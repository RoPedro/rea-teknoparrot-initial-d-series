//! Golden snapshots (rea-launcher-sdk docs/STANDARDS.md, "Golden
//! snapshots"): the settings files, the launch plan of every game on both
//! hosts, the runtime folder plans and windowsarcadeloader's input lines,
//! compared with the files in `tests/snapshots/`, so an SDK bump or a
//! refactor can't change them silently. A diff is reviewed like code: it
//! must match an intended change or an SDK Migration item.
//!
//! `REA_UPDATE_SNAPSHOTS=1 cargo test --test snapshots` rewrites them.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use initiald_core::catalog::{GAMES, Game, Host};
use initiald_core::input::{Button, StickLayout};
use initiald_core::launch::{self, Components, Places};
use initiald_core::runtime::{self, Item, Op};
use initiald_core::settings::{Crt, Settings, Size, Upscaler};
use rea_launcher::wheel::{Control, Controls};

/// A path as the snapshot spells it on every system (Windows joins with `\`).
fn p(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[test]
fn the_settings_template() {
    rea_launcher::snapshot!("settings-template.cfg.txt", Settings::default().save(""));
}

/// A player's file: comments, a commented-out key, unknown keys, a bad
/// value, a key twice, a line without `=`; then the same with a UTF-8 BOM.
#[test]
fn a_players_settings_file_round_trips() {
    let file = "# my notes\n\
                upscaler = nis\n\
                # crt = lottes\n\
                future_key = 7\n\
                output=2560x1440\n\
                sharpness = 50\n\
                render = native\n\
                deadzone = 0.123\n\
                fullscreen = off\n\
                no equals sign\n\
                upscaler = linear\n";
    let mut out = String::new();
    for (name, text) in [
        ("plain", file.to_string()),
        ("bom", format!("\u{feff}{file}")),
    ] {
        let (s, problems) = Settings::parse(&text);
        let _ = writeln!(out, "== {name}\n{s:?}");
        for problem in problems {
            let _ = writeln!(out, "problem {problem}");
        }
        let _ = write!(out, "-- saved\n{}", s.save(&text));
    }
    rea_launcher::snapshot!("settings-round-trip.txt", out);
}

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

/// The settings each game's plan is shown with.
fn variants() -> Vec<(&'static str, Settings, bool)> {
    let custom = Settings {
        render: Some(Size::new(2560, 1440)),
        output: Size::new(1920, 1080),
        upscaler: Upscaler::Nis,
        sharpness: 12,
        crt: Crt::Lottes,
        fullscreen: false,
        deadzone: 0.2,
    };
    let linear = Settings {
        upscaler: Upscaler::Linear,
        ..Settings::default()
    };
    vec![
        ("defaults", Settings::default(), false),
        ("custom", custom.clone(), false),
        ("linear", linear, false),
        ("headless", custom, true),
    ]
}

fn write_plan(out: &mut String, plan: &launch::Plan) {
    let _ = writeln!(out, "size {}", plan.game_size);
    let _ = writeln!(out, "program {}", p(&plan.game.program));
    let _ = writeln!(out, "args {}", plan.game.args.join(" ").replace('\\', "/"));
    let _ = writeln!(out, "cwd {}", p(&plan.game.cwd));
    for (k, v) in &plan.game.env {
        let _ = writeln!(out, "env {k}={}", v.replace('\\', "/"));
    }
    let _ = writeln!(out, "gamescope {}", plan.gamescope.join(" "));
    if let Some(home) = &plan.gamescope_home {
        let _ = writeln!(out, "gamescope home {}", p(home));
    }
    let _ = writeln!(out, "input {:?}", plan.input_pipe);
    for (at, target) in &plan.links {
        let _ = writeln!(out, "link {} -> {}", p(at), p(target));
    }
    for (f, text) in &plan.files {
        let _ = write!(out, "file {}\n{text}", p(f));
        if !text.ends_with('\n') {
            out.push('\n');
        }
    }
}

/// What each game plans (files, links, command, environment, gamescope)
/// for each settings variant, on Linux and Windows, with and without the
/// NAOMI 2 BIOS.
#[test]
fn every_plan_of_every_game() {
    let c = components();
    let mut out = String::new();
    for host in [Host::Linux, Host::Windows] {
        for g in GAMES {
            for bios in [true, false] {
                let places = Places {
                    runtime: PathBuf::from("/d/runtimes").join(g.slug),
                    wal_root: "/d/wal".into(),
                    naomi2_bios: bios.then(|| "/bios/naomi2.zip".into()),
                    wal_port: 33700,
                };
                let entry = PathBuf::from("/lib").join(g.slug);
                for (name, s, headless) in variants() {
                    // the BIOS matters only to the NAOMI 2 games' defaults
                    if !bios && (name != "defaults" || g.board != "Sega NAOMI 2") {
                        continue;
                    }
                    let what = if bios { name } else { "defaults, no BIOS" };
                    let _ = writeln!(out, "== {host:?} {} {what}", g.slug);
                    let plan = launch::plan(g, &entry, &places, &c, &s, "eth0", headless, host);
                    write_plan(&mut out, &plan);
                }
            }
        }
    }
    rea_launcher::snapshot!("plans.txt", out);
}

/// A game folder holding a bit of everything the runtime rules look at.
fn listing(g: &Game) -> Vec<Item> {
    let item = |name: &str, dir: bool, size: u64| Item {
        name: name.into(),
        dir,
        size,
    };
    let main = g.main_file.split('/').next().expect("a first component");
    vec![
        item("$RECYCLE.BIN", true, 0),
        item("AMFS", true, 0),
        item("AppData", true, 0),
        item("Card_01.crd", false, 100),
        item("System Volume Information", true, 0),
        item("WindowsLoader", true, 0),
        item("Y", true, 0),
        item("amfs", true, 0),
        item("big.bin", false, 64 << 20),
        item("controls.ini", false, 100),
        item("data", true, 0),
        item("eeprom.bin", false, 100),
        item("hook.dll", false, 100),
        item("lindbergh.ini", false, 100),
        item(main, main != g.main_file, 30 << 20),
        item("old.windowsloader", false, 10),
        item("playaccount.dat", false, 100),
        item("small.manifest", false, 100),
        item("sram.bin", false, 100),
        item("Setting.ini", false, 4 * 1024 * 1024 - 1),
        item("exactly4m.bin", false, 4 * 1024 * 1024),
    ]
}

/// The runtime folder each game gets from one listing of its folder.
#[test]
fn every_runtime_plan() {
    let mut out = String::new();
    for g in GAMES {
        let _ = writeln!(out, "== {}", g.slug);
        let src = PathBuf::from("/lib").join(g.slug);
        let rt = PathBuf::from("/rt").join(g.slug);
        for op in runtime::plan(g, &src, &listing(g), &rt) {
            let _ = match op {
                Op::Dir(d) => writeln!(out, "dir {}", p(&d)),
                Op::Link { target, at } => writeln!(out, "link {} -> {}", p(&at), p(&target)),
                Op::CopyOnce { from, to } => writeln!(out, "copy once {} <- {}", p(&to), p(&from)),
            };
        }
        let _ = writeln!(
            out,
            "game folder {:?}; loader path {:?}; first start marker {:?}",
            g.game_folder(),
            g.loader_path(),
            g.first_start_marker()
        );
    }
    rea_launcher::snapshot!("runtime-plans.txt", out);
}

/// windowsarcadeloader's input line for each button alone and for the
/// axes, in both stick layouts.
#[test]
fn every_input_line() {
    let mut out = String::new();
    for layout in [StickLayout::RingEdge, StickLayout::Alls] {
        let _ = writeln!(out, "== {layout:?}");
        for &b in Button::ALL {
            let read = Controls {
                buttons: b.bit(),
                ..Controls::default()
            };
            let _ = writeln!(out, "{:<10} {}", b.key(), layout.pipe_line(&read));
        }
        for (steer, accel, brake) in [(-1.5, 0.5, 0.0), (1.0, 1.0, 1.0), (0.25, -1.0, 2.0)] {
            let read = Controls {
                steer,
                accel,
                brake,
                buttons: 0,
            };
            let _ = writeln!(
                out,
                "steer {steer} accel {accel} brake {brake}: {}",
                layout.pipe_line(&read)
            );
        }
    }
    rea_launcher::snapshot!("input-lines.txt", out);
}
