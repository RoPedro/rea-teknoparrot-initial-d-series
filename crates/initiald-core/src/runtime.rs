//! The private runtime folder each game runs from: which entries of the
//! player's game folder are linked, copied or left out
//! (`rea_launcher::linkfarm` builds it and never touches the game folder).
//!
//! The runtime holds a link to each top-level entry of the game folder,
//! plus the files the launcher writes and the folders the loaders create
//! (saves, cabinet drives) as real files.

use std::io;
use std::path::Path;

use rea_launcher::linkfarm::{self, Item, Op, Place, Policy};

use crate::catalog::{Game, Route};

/// Top-level files of a Windows game below this size are copied, not
/// linked: the games write their logs and account files there
/// (Zero's `IniD0_AppError.log`, `playaccount.dat`), and the library is
/// read-only while they run.
const SMALL: u64 = 4 * 1024 * 1024;

/// A game's rules for its runtime folder.
struct Rules(Route);

impl Policy for Rules {
    fn place(&self, name: &str, it: &Item) -> Place {
        if linkfarm::ntfs_leftover(name) || loader_owned(self.0, name) {
            Place::Skip
        } else if matches!(self.0, Route::Wal { .. }) && !it.dir && it.size < SMALL && !code(name) {
            Place::CopyOnce
        } else {
            Place::Link
        }
    }
}

fn code(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [".exe", ".dll", ".manifest"]
        .iter()
        .any(|x| name.ends_with(x))
}

/// Names never linked: files the launcher or the loaders write in the
/// runtime.
fn loader_owned(route: Route, name: &str) -> bool {
    match route {
        Route::Lindbergh => matches!(
            name,
            "lindbergh.ini"
                | "controls.ini"
                | "sram.bin"
                | "eeprom.bin"
                | "Card_01.crd"
                | "Card_02.crd"
        ),
        // windowsarcadeloader's dump marker and the cabinet drives it creates
        Route::Wal { .. } => {
            name.ends_with(".windowsloader")
                || matches!(name, "AMFS" | "amfs" | "AppData" | "Y" | "WindowsLoader")
        }
        Route::Flycast { .. } => false,
    }
}

/// The runtime `rt` of a game in folder `src` (`ls` lists a folder:
/// `linkfarm::list`). A Lindbergh or Windows game's entries go in its
/// [`Game::game_folder`]; a NAOMI 2 game needs only the folder.
pub fn plan(
    game: &Game,
    src: &Path,
    rt: &Path,
    ls: &dyn Fn(&Path) -> io::Result<Vec<Item>>,
) -> io::Result<Vec<Op>> {
    let mut ops = vec![Op::Dir(rt.to_path_buf())];
    if !matches!(game.route, Route::Flycast { .. }) {
        ops.extend(linkfarm::plan(
            src,
            &rt.join(game.game_folder()),
            &Rules(game.route),
            ls,
        )?);
        ops.dedup();
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GameId, game};
    use std::path::PathBuf;

    fn items(names: &[&str]) -> impl Fn(&Path) -> io::Result<Vec<Item>> {
        let v: Vec<Item> = names
            .iter()
            .map(|n| Item {
                name: n.to_string(),
                dir: false,
                size: if n.ends_with(".exe") { 30 << 20 } else { 100 },
            })
            .collect();
        move |_| Ok(v.clone())
    }

    #[test]
    fn wal_runtime_links_the_dump_but_not_drives_or_markers() {
        let ls = items(&[
            "InitialD0_DX11_Nu.exe",
            "AppData",
            "old.windowsloader",
            "data",
        ]);
        let ops = plan(
            game(GameId::Zero),
            Path::new("/lib/zero"),
            Path::new("/rt/zero"),
            &ls,
        )
        .unwrap();
        // Zero's files go in package\, as on the cabinet
        assert_eq!(
            ops,
            [
                Op::Dir("/rt/zero".into()),
                Op::Dir("/rt/zero/package".into()),
                Op::Link {
                    target: "/lib/zero/InitialD0_DX11_Nu.exe".into(),
                    at: "/rt/zero/package/InitialD0_DX11_Nu.exe".into()
                },
                Op::CopyOnce {
                    from: "/lib/zero/data".into(),
                    to: "/rt/zero/package/data".into()
                },
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn small_files_of_a_windows_game_are_copied_once_and_kept() {
        use std::fs;
        let t = tempfile::tempdir().unwrap();
        let (src, rt) = (t.path().join("dump"), t.path().join("rt"));
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("IniD0_AppError.log"), b"old").unwrap();
        fs::write(src.join("idzhook.dll"), b"dll").unwrap();
        let ops = plan(game(GameId::Zero), &src, &rt, &linkfarm::list).unwrap();
        let pkg = rt.join("package");
        // an older launcher linked the log: replaced by a copy
        fs::create_dir_all(&pkg).unwrap();
        std::os::unix::fs::symlink(
            src.join("IniD0_AppError.log"),
            pkg.join("IniD0_AppError.log"),
        )
        .unwrap();
        linkfarm::apply(&rt, &ops, linkfarm::Occupied::Refuse).unwrap();
        let log = pkg.join("IniD0_AppError.log");
        assert!(!fs::symlink_metadata(&log).unwrap().file_type().is_symlink());
        assert!(
            fs::symlink_metadata(pkg.join("idzhook.dll"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::write(&log, b"the game's").unwrap();
        linkfarm::apply(&rt, &ops, linkfarm::Occupied::Refuse).unwrap();
        assert_eq!(fs::read(&log).unwrap(), b"the game's");
        assert_eq!(fs::read(src.join("IniD0_AppError.log")).unwrap(), b"old");
    }

    #[test]
    fn lindbergh_runtime_keeps_the_launcher_files_its_own() {
        let ls = items(&["id5.elf", "lindbergh.ini", "System Volume Information"]);
        let ops = plan(
            game(GameId::Id5),
            Path::new("/lib/id5"),
            Path::new("/rt/id5"),
            &ls,
        )
        .unwrap();
        assert_eq!(
            ops,
            [
                Op::Dir("/rt/id5".into()),
                Op::Link {
                    target: "/lib/id5/id5.elf".into(),
                    at: "/rt/id5/id5.elf".into()
                }
            ]
        );
    }

    #[test]
    fn a_naomi_2_game_needs_only_its_folder() {
        let unlisted = |_: &Path| -> io::Result<Vec<Item>> { panic!("not listed") };
        let rt = PathBuf::from("/rt/id1");
        let ops = plan(game(GameId::Id1), Path::new("/lib/naomi2"), &rt, &unlisted).unwrap();
        assert_eq!(ops, [Op::Dir(rt)]);
    }
}
