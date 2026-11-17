//! Golden snapshots of a session's Linux side (rea-launcher-sdk
//! docs/STANDARDS.md, "Golden snapshots"): the command line of every game
//! (gamescope, bubblewrap, the game) and the runtime folder [`prepare`]
//! builds, compared with `tests/snapshots/`. Rewrite them with
//! `REA_UPDATE_SNAPSHOTS=1 cargo test` and review the diff like code.

use std::fmt::Write as _;
use std::path::Path;

use initiald_core::catalog::{GAMES, GameId, game};
use initiald_core::launch::Components;
use initiald_core::library::Found;
use initiald_core::settings::{Crt, Size, Upscaler};

use super::*;

fn components() -> Components {
    Components {
        lindbergh: "/c/lindbergh".into(),
        wal: "/c/wal".into(),
        runner: "/c/runner".into(),
        winetricks: "/c/winetricks".into(),
        flycast: "/c/flycast".into(),
        shaders: "/c/shaders".into(),
    }
}

/// Every game's command line, with the default settings, with a CRT look
/// and a render size, headless, and without bubblewrap.
#[test]
fn every_command_line() {
    let crt = Settings {
        render: Some(Size::new(2560, 1440)),
        upscaler: Upscaler::Nis,
        crt: Crt::Lottes,
        ..Settings::default()
    };
    let libraries = ["/lib".into(), "/naomi".into()];
    let mut out = String::new();
    for g in GAMES {
        let places = Places {
            runtime: Path::new("/d/runtimes").join(g.slug),
            wal_root: "/d/wal".into(),
            naomi2_bios: Some("/bios/naomi2.zip".into()),
            wal_port: DEFAULT_WAL_PORT,
        };
        let entry = Path::new("/lib").join(g.slug);
        for (name, s, headless, bwrap) in [
            ("defaults", Settings::default(), false, true),
            ("crt", crt.clone(), false, true),
            ("headless", Settings::default(), true, true),
            ("no bwrap", Settings::default(), false, false),
        ] {
            let plan = launch::plan(
                g,
                &entry,
                &places,
                &components(),
                &s,
                "eth0",
                headless,
                Host::Linux,
            );
            let argv = command_line(&plan, &libraries, bwrap, "/home/p");
            let _ = writeln!(out, "{} {name}: {}", g.slug, argv.join(" "));
        }
    }
    rea_launcher::snapshot!("command-lines.txt", out);
}

/// Every entry under `dir`, sorted: folders, links and their targets,
/// files and their text.
fn tree(dir: &Path, root: &Path, out: &mut String) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let rel = path.strip_prefix(root).unwrap().display().to_string();
        let meta = fs::symlink_metadata(&path).unwrap();
        if meta.file_type().is_symlink() {
            let target = fs::read_link(&path).unwrap();
            let target = target.strip_prefix(root).unwrap_or(&target);
            let _ = writeln!(out, "link {rel} -> <tmp>/{}", target.display());
        } else if meta.is_dir() {
            let _ = writeln!(out, "dir  {rel}");
            tree(&path, root, out);
        } else {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let _ = write!(out, "file {rel}\n{text}");
            if !text.is_empty() && !text.ends_with('\n') {
                out.push('\n');
            }
        }
    }
}

/// The lines that depend on the machine: the network interface and the
/// free port.
fn machine_free(text: &str) -> String {
    text.lines()
        .map(|l| {
            if l.starts_with("NIC_NAME = ") {
                "NIC_NAME = <nic>".to_string()
            } else if l.starts_with("port: ") {
                "port: <free>".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// What [`prepare`] builds for a Lindbergh game and three Windows games on
/// Linux, from a synthetic game folder and empty stand-ins for the
/// components.
#[test]
fn prepare_builds_the_runtime_folders() {
    let t = tempfile::tempdir().unwrap();
    let paths = Paths {
        data: t.path().join("data"),
        components: t.path().join("components"),
    };
    let c = paths.components();
    for dir in [
        c.wal.join("payloads"),
        c.wal.join("systemprofiles"),
        c.runner.join("bin"),
    ] {
        fs::create_dir_all(dir).unwrap();
    }
    for file in [c.wal.join("arcade-launcher"), c.runner.join("bin/wine")] {
        fs::write(file, b"").unwrap();
    }
    fs::create_dir_all(&c.lindbergh).unwrap();
    for file in ["lindbergh", "lindbergh.so"] {
        fs::write(c.lindbergh.join(file), b"").unwrap();
    }
    let mut out = String::new();
    for id in [GameId::Id5, GameId::Id6, GameId::Zero, GameId::TheArcade5] {
        let g = game(id);
        let dir = t.path().join("games").join(g.slug);
        fs::create_dir_all(dir.join("data")).unwrap();
        let main = dir.join(g.main_file);
        fs::create_dir_all(main.parent().unwrap()).unwrap();
        fs::write(main, b"main").unwrap();
        fs::write(dir.join("Setting.ini"), b"[game]\n").unwrap();
        let entry = Entry {
            dir,
            found: Found::Known {
                game: g,
                edition: &g.editions[0],
            },
        };
        let prepared = prepare(&paths, g, &entry, &Settings::default(), Display::Headless).unwrap();
        let _ = writeln!(
            out,
            "== {}\nfirst start marker {:?}",
            g.slug,
            prepared
                .first_start
                .map(|m| m.strip_prefix(t.path()).unwrap().display().to_string())
        );
    }
    tree(&paths.data, t.path(), &mut out);
    rea_launcher::snapshot!("prepared-runtimes.txt", machine_free(&out));
}
