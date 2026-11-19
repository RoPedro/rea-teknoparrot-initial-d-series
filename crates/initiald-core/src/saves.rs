//! Per-game card and progress saves, kept together under `<data>/saves/<slug>/`
//! so a player can back up every Initial D game's cards, Aime profiles and
//! cabinet backup memory in one folder.
//!
//! Uses [`rea_launcher::saves::Mirror`] with recorded sync state:
//! - before a launch, files the player changed or restored in `saves/<slug>/`
//!   win even with older timestamps; files only the runtime changed (after an
//!   abrupt stop) are preserved into `saves/<slug>/`;
//! - after exit, every card or save file the game wrote or updated is copied
//!   atomically into `saves/<slug>/`;
//! - nothing is ever deleted, and existing runtimes migrate automatically into
//!   `saves/<slug>/` on their next start.

use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::catalog::Host;
use rea_launcher::saves::{Deleted, FirstRun, Mirror, Moved};

use crate::catalog::{Game, Route};

/// The folder inside `runtime` where `game` keeps its card and save files.
pub fn runtime_dir(game: &Game, runtime: &Path, host: Host) -> PathBuf {
    match game.route {
        Route::Flycast { .. } => match host {
            Host::Linux => runtime.join("data/flycast"),
            Host::Windows => runtime.join("flycast/data"),
        },
        Route::Lindbergh => runtime.to_path_buf(),
        Route::Wal {
            system: "ringedge", ..
        } => runtime.join("sv"),
        Route::Wal { .. } => runtime.join("AppData"),
    }
}

/// Whether `rel` (relative to [`runtime_dir`], `/` separators) is a card or
/// progress save file of `game` that belongs in `saves/<slug>/`.
pub fn include(game: &Game, rel: &str) -> bool {
    let lower = rel.replace('\\', "/").to_ascii_lowercase();
    match game.route {
        Route::Flycast { .. } => {
            // Magnetic card (`*.card`), cabinet NVRAM/EEPROM; skip BIOS zips and boxart.
            !lower.contains('/')
                && [".card", ".nvmem", ".eeprom", ".sram", ".bin"]
                    .iter()
                    .any(|ext| lower.ends_with(ext))
        }
        Route::Lindbergh => {
            // ID4 (`InidCrd000.crd`), ID5 (`InidCard00.crd`), generic cards, SRAM/EEPROM.
            !lower.contains('/')
                && (lower == "sram.bin"
                    || lower == "eeprom.bin"
                    || (lower.ends_with(".crd")
                        && (lower.starts_with("inidcrd")
                            || lower.starts_with("inidcard")
                            || lower.starts_with("card_"))))
        }
        Route::Wal {
            system: "ringedge", ..
        } => {
            // Everything in `sv/`: RFID card (`card.ic`/`card.bin`), `SRAM.bin`, `E2PROM.bin`, `AppUser/*`.
            !lower.is_empty()
        }
        Route::Wal {
            system: "seganu", ..
        } => {
            // Zero Ver. 2 local game server: `AppData/idz/*`, `AppData/idz2/*` (profiles, teams).
            lower.starts_with("idz/") || lower.starts_with("idz2/")
        }
        Route::Wal { system: "alls", .. } => {
            // THE ARCADE S3/S5: `AppData/idac/*` (Aime profiles, garage, story, rankings),
            // `AppData/SDGT/appfile.dat`, ghosts and avatars; skip crash dumps and logs.
            lower.starts_with("idac/")
                || lower == "sdgt/appfile.dat"
                || lower.starts_with("sdgt/ghost/")
                || (lower.starts_with("sdgt/avatarimage_") && lower.ends_with(".png"))
        }
        Route::Wal { .. } => false,
    }
}

fn with_mirror<T>(
    game: &Game,
    runtime: &Path,
    saves_dir: &Path,
    host: Host,
    f: impl for<'a> FnOnce(&Mirror<'a>) -> io::Result<T>,
) -> io::Result<T> {
    let rt_dir = runtime_dir(game, runtime, host);
    let state = runtime.join(".saves-state");
    let keep = |rel: &str| include(game, rel);
    let mut mirror = Mirror::folder(&rt_dir, saves_dir, &state, FirstRun::SavesWin)
        .deleted(Deleted::Restore)
        .only(&keep);
    // Lindbergh's runtime root holds symlinks to multi-GB game folders;
    // Flycast's data folder holds `boxart/`. Only scan their top level.
    if matches!(game.route, Route::Lindbergh | Route::Flycast { .. }) {
        mirror = mirror.scan(&[""]);
    }
    f(&mirror)
}

/// Mirrors `game`'s saves and cards before the game starts.
pub fn sync_before(game: &Game, runtime: &Path, saves_dir: &Path, host: Host) -> io::Result<Moved> {
    with_mirror(game, runtime, saves_dir, host, |m| m.before())
}

/// Copies card and save changes written by `game` back into `saves_dir`.
pub fn sync_after(
    game: &Game,
    runtime: &Path,
    saves_dir: &Path,
    host: Host,
) -> io::Result<Vec<String>> {
    with_mirror(game, runtime, saves_dir, host, |m| m.after())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GameId, game};
    use std::fs;

    #[test]
    fn each_route_keeps_cards_and_saves_and_skips_junk() {
        let id1 = game(GameId::Id1);
        assert!(include(id1, "initdexp.zip.card"));
        assert!(include(id1, "initdexp.zip.nvmem"));
        assert!(include(id1, "initdexp.zip.eeprom"));
        assert!(!include(id1, "naomi2.zip"));
        assert!(!include(id1, "boxart/initdexp.png"));

        let id4 = game(GameId::Id4);
        assert!(include(id4, "InidCrd000.crd"));
        assert!(include(id4, "InidCard00.crd"));
        assert!(include(id4, "sram.bin"));
        assert!(include(id4, "eeprom.bin"));
        assert!(!include(id4, "lindbergh.ini"));
        assert!(!include(id4, "id4.elf"));

        let id8 = game(GameId::Id8);
        assert!(include(id8, "card.ic"));
        assert!(include(id8, "SRAM.bin"));
        assert!(include(id8, "E2PROM.bin"));
        assert!(include(id8, "AppUser/profile.dat"));

        let zero = game(GameId::Zero);
        assert!(include(zero, "idz2/1.json"));
        assert!(include(zero, "idz2/teams.json"));
        assert!(include(zero, "idz/1.json"));
        assert!(!include(zero, "amdaemon.log"));

        let ta5 = game(GameId::TheArcade5);
        assert!(include(ta5, "idac/0.json"));
        assert!(include(ta5, "SDGT/appfile.dat"));
        assert!(include(ta5, "SDGT/GHOST/course1.bin"));
        assert!(include(ta5, "SDGT/AvatarImage_0000.png"));
        assert!(!include(ta5, "tmp.dmp"));
        assert!(!include(ta5, "SDGT/abaaslog/log.txt"));
        assert!(!include(ta5, "amdaemon.log"));
    }

    #[test]
    fn round_trip_mirrors_cards_out_and_restores_player_backup() {
        let t = tempfile::tempdir().unwrap();
        let rt = t.path().join("runtimes/id8");
        let sv = rt.join("sv");
        let saves = t.path().join("saves/id8");
        fs::create_dir_all(&sv).unwrap();

        // First run: game creates card.ic and SRAM.bin in runtime
        sync_before(game(GameId::Id8), &rt, &saves, Host::Linux).unwrap();
        fs::write(sv.join("card.ic"), b"car-ae86-v1").unwrap();
        fs::write(sv.join("SRAM.bin"), b"sram-v1").unwrap();
        let after = sync_after(game(GameId::Id8), &rt, &saves, Host::Linux).unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(fs::read(saves.join("card.ic")).unwrap(), b"car-ae86-v1");
        assert_eq!(fs::read(saves.join("SRAM.bin")).unwrap(), b"sram-v1");

        // Player restores a backup into saves/id8/card.ic -> next launch wins
        fs::write(saves.join("card.ic"), b"car-rx7-backup").unwrap();
        let before = sync_before(game(GameId::Id8), &rt, &saves, Host::Linux).unwrap();
        assert_eq!(before.to_runtime, vec!["card.ic"]);
        assert_eq!(fs::read(sv.join("card.ic")).unwrap(), b"car-rx7-backup");
    }

    #[test]
    fn existing_runtime_saves_migrate_into_saves_folder_on_first_sync() {
        let t = tempfile::tempdir().unwrap();
        let rt = t.path().join("runtimes/ta5");
        let idac = rt.join("AppData/idac");
        let saves = t.path().join("saves/ta5");
        fs::create_dir_all(&idac).unwrap();
        fs::write(idac.join("0.json"), br#"{"name":"TAKUMI"}"#).unwrap();
        fs::write(rt.join("AppData/tmp.dmp"), b"crash").unwrap();

        let moved = sync_before(game(GameId::TheArcade5), &rt, &saves, Host::Linux).unwrap();
        assert_eq!(moved.to_saves, vec!["idac/0.json"]);
        assert_eq!(
            fs::read(saves.join("idac/0.json")).unwrap(),
            br#"{"name":"TAKUMI"}"#
        );
        assert!(!saves.join("tmp.dmp").exists());
    }

    #[test]
    fn flycast_uses_os_data_dir_and_skips_bios_and_boxart() {
        let t = tempfile::tempdir().unwrap();
        let rt = t.path().join("runtimes/id2");
        let data = rt.join("data/flycast");
        let saves = t.path().join("saves/id2");
        fs::create_dir_all(data.join("boxart")).unwrap();
        fs::write(data.join("initdv2e.zip.card"), b"card-data").unwrap();
        fs::write(data.join("initdv2e.zip.eeprom"), b"eeprom-data").unwrap();
        fs::write(data.join("naomi2.zip"), b"bios").unwrap();
        fs::write(data.join("boxart/initdv2e.png"), b"img").unwrap();

        sync_before(game(GameId::Id2), &rt, &saves, Host::Linux).unwrap();
        assert_eq!(
            fs::read(saves.join("initdv2e.zip.card")).unwrap(),
            b"card-data"
        );
        assert_eq!(
            fs::read(saves.join("initdv2e.zip.eeprom")).unwrap(),
            b"eeprom-data"
        );
        assert!(!saves.join("naomi2.zip").exists());
        assert!(!saves.join("boxart").exists());
        assert_eq!(
            runtime_dir(game(GameId::Id2), &rt, Host::Windows),
            rt.join("flycast/data")
        );
    }

    #[test]
    fn lindbergh_scans_only_top_level_and_never_touches_game_dirs() {
        let t = tempfile::tempdir().unwrap();
        let rt = t.path().join("runtimes/id4");
        let saves = t.path().join("saves/id4");
        fs::create_dir_all(rt.join("data")).unwrap();
        fs::write(rt.join("InidCrd000.crd"), vec![0u8; 1824]).unwrap();
        fs::write(rt.join("sram.bin"), b"sram").unwrap();
        fs::write(rt.join("lindbergh.ini"), b"ini").unwrap();
        fs::write(rt.join("data/InidCrd999.crd"), b"nested").unwrap();

        sync_before(game(GameId::Id4), &rt, &saves, Host::Linux).unwrap();
        assert!(saves.join("InidCrd000.crd").is_file());
        assert!(saves.join("sram.bin").is_file());
        assert!(!saves.join("lindbergh.ini").exists());
        assert!(!saves.join("data").exists());
    }
}
