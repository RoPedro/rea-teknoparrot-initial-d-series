//! The private runtime folder each game runs from.
//!
//! The player's game folder is read-only (it may be on a network share,
//! and the launcher runs every game with the library bound read-only).
//! The runtime holds a symlink to each top-level entry of the game folder,
//! plus the files the launcher writes and the folders the loaders create
//! (saves, cabinet drives) as real files. [`plan`] decides from a listing;
//! [`apply`] carries it out without ever touching the game folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::catalog::{Game, Route};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Dir(PathBuf),
    /// Symlink `at` → `target` (replaced when it points elsewhere).
    Link {
        target: PathBuf,
        at: PathBuf,
    },
    /// A copy the game owns from then on: made once (replacing a link an
    /// older launcher left), never overwritten.
    CopyOnce {
        from: PathBuf,
        to: PathBuf,
    },
}

/// Top-level files of a Windows game below this size are copied, not
/// linked: the games write their logs and account files there
/// (Zero's `IniD0_AppError.log`, `playaccount.dat`), and the library is
/// read-only while they run.
const SMALL: u64 = 4 * 1024 * 1024;

fn copied(route: Route, it: &Item) -> bool {
    let code = [".exe", ".dll", ".manifest"]
        .iter()
        .any(|x| it.name.to_ascii_lowercase().ends_with(x));
    matches!(route, Route::Wal { .. }) && !it.dir && it.size < SMALL && !code
}

/// One entry of the game folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub dir: bool,
    pub size: u64,
}

pub fn list(dir: &Path) -> io::Result<Vec<Item>> {
    let mut v = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let m = fs::metadata(e.path())?;
        v.push(Item {
            name: e.file_name().to_string_lossy().into_owned(),
            dir: m.is_dir(),
            size: m.len(),
        });
    }
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

/// Names never linked: files the launcher or the loaders write in the
/// runtime, and NTFS leftovers in the dumps.
fn skip(route: Route, name: &str) -> bool {
    if name == "System Volume Information" || name.starts_with('$') {
        return true;
    }
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

/// The runtime of a Lindbergh or Windows game: every entry of the game
/// folder linked. NAOMI 2 games need no runtime folder of this kind.
pub fn plan(game: &Game, src: &Path, items: &[Item], rt: &Path) -> Vec<Op> {
    let mut ops = vec![Op::Dir(rt.to_path_buf())];
    if matches!(game.route, Route::Flycast { .. }) {
        return ops;
    }
    let dest = rt.join(game.game_folder());
    if dest != rt {
        ops.push(Op::Dir(dest.clone()));
    }
    for it in items {
        if skip(game.route, &it.name) {
            continue;
        }
        let (from, to) = (src.join(&it.name), dest.join(&it.name));
        if copied(game.route, it) {
            ops.push(Op::CopyOnce { from, to });
        } else {
            ops.push(Op::Link {
                target: from,
                at: to,
            });
        }
    }
    ops
}

/// Carry out a plan. A link is replaced only when it points elsewhere; a
/// real file or folder where a link should be is an error (it would be
/// the player's data), never removed.
pub fn apply(ops: &[Op]) -> io::Result<()> {
    for op in ops {
        match op {
            Op::Dir(d) => fs::create_dir_all(d)?,
            Op::Link { target, at } => match fs::symlink_metadata(at) {
                Ok(m) if m.file_type().is_symlink() => {
                    if fs::read_link(at)? != *target {
                        fs::remove_file(at)?;
                        symlink(target, at)?;
                    }
                }
                Ok(_) => {
                    return Err(io::Error::other(format!(
                        "{} is a real file or folder where the launcher expects a link; move it away",
                        at.display()
                    )));
                }
                Err(_) => symlink(target, at)?,
            },
            Op::CopyOnce { from, to } => match fs::symlink_metadata(to) {
                Ok(m) if !m.file_type().is_symlink() => {}
                found => {
                    if found.is_ok() {
                        fs::remove_file(to)?;
                    }
                    fs::copy(from, to)?;
                }
            },
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(target: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(not(unix))]
fn symlink(_target: &Path, _at: &Path) -> io::Result<()> {
    Err(io::Error::other("symlinks are only made on Linux"))
}

/// Links in a runtime folder whose target is gone (a dump moved): removed
/// so a re-plan can link the new place.
pub fn dangling(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut v = Vec::new();
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if fs::symlink_metadata(&p)?.file_type().is_symlink() && fs::metadata(&p).is_err() {
            v.push(p);
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GameId, game};

    fn items(names: &[&str]) -> Vec<Item> {
        names
            .iter()
            .map(|n| Item {
                name: n.to_string(),
                dir: false,
                size: if n.ends_with(".exe") { 30 << 20 } else { 100 },
            })
            .collect()
    }

    #[test]
    fn wal_runtime_links_the_dump_but_not_drives_or_markers() {
        let g = game(GameId::Zero);
        let ops = plan(
            g,
            Path::new("/lib/zero"),
            &items(&[
                "InitialD0_DX11_Nu.exe",
                "AppData",
                "old.windowsloader",
                "data",
            ]),
            Path::new("/rt/zero"),
        );
        let linked: Vec<_> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Link { at, .. } => Some(at.file_name().unwrap().to_str().unwrap()),
                _ => None,
            })
            .collect();
        assert_eq!(linked, ["InitialD0_DX11_Nu.exe"]);
        // Zero's files go in package\, as on the cabinet
        assert!(ops.iter().any(|o| matches!(o, Op::Link { at, .. }
            if at == Path::new("/rt/zero/package/InitialD0_DX11_Nu.exe"))));
        assert!(
            ops.iter()
                .any(|o| matches!(o, Op::CopyOnce { to, .. } if to.ends_with("data")))
        );
    }

    #[test]
    fn small_files_of_a_windows_game_are_copied_once_and_kept() {
        let t = tempfile::tempdir().unwrap();
        let (src, rt) = (t.path().join("dump"), t.path().join("rt"));
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("IniD0_AppError.log"), b"old").unwrap();
        fs::write(src.join("idzhook.dll"), b"dll").unwrap();
        let ops = plan(game(GameId::Zero), &src, &list(&src).unwrap(), &rt);
        let pkg = rt.join("package");
        // an older launcher linked the log: replaced by a copy
        fs::create_dir_all(&pkg).unwrap();
        symlink(
            &src.join("IniD0_AppError.log"),
            &pkg.join("IniD0_AppError.log"),
        )
        .unwrap();
        apply(&ops).unwrap();
        let log = pkg.join("IniD0_AppError.log");
        assert!(!fs::symlink_metadata(&log).unwrap().file_type().is_symlink());
        assert!(
            fs::symlink_metadata(pkg.join("idzhook.dll"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::write(&log, b"the game's").unwrap();
        apply(&ops).unwrap();
        assert_eq!(fs::read(&log).unwrap(), b"the game's");
        assert_eq!(fs::read(src.join("IniD0_AppError.log")).unwrap(), b"old");
    }

    #[test]
    fn lindbergh_runtime_keeps_the_launcher_files_its_own() {
        let ops = plan(
            game(GameId::Id5),
            Path::new("/lib/id5"),
            &items(&["id5.elf", "lindbergh.ini", "System Volume Information"]),
            Path::new("/rt/id5"),
        );
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn apply_links_relinks_and_never_replaces_real_files() {
        let t = tempfile::tempdir().unwrap();
        let (a, b, rt) = (t.path().join("a"), t.path().join("b"), t.path().join("rt"));
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let at = rt.join("data");
        apply(&[
            Op::Dir(rt.clone()),
            Op::Link {
                target: a.clone(),
                at: at.clone(),
            },
        ])
        .unwrap();
        apply(&[Op::Link {
            target: b.clone(),
            at: at.clone(),
        }])
        .unwrap();
        assert_eq!(fs::read_link(&at).unwrap(), b);
        fs::write(rt.join("save.bin"), b"mine").unwrap();
        let err = apply(&[Op::Link {
            target: a,
            at: rt.join("save.bin"),
        }]);
        assert!(err.is_err());
        assert_eq!(fs::read(rt.join("save.bin")).unwrap(), b"mine");
    }
}
