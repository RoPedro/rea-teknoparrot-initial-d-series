//! Golden snapshots of the window (rea-launcher-sdk docs/STANDARDS.md,
//! "Golden snapshots"): what a screen reader finds on each page and in
//! each important state — role, name, value, description, disabled or
//! selected, and where it is — at 1280×720 and 1280×800, compared with
//! `tests/snapshots/ui/`. The games' states depend on the host (the
//! Lindbergh games are Linux-only), so each host has its own files.
//! Rewrite them with `REA_UPDATE_SNAPSHOTS=1 cargo test` and review the
//! diff like code.

use std::fmt::Write as _;

use super::*;

const SIZES: [Vec2; 2] = [vec2(1280.0, 720.0), vec2(1280.0, 800.0)];

/// One line per node a player can perceive, in tree order: text runs (the
/// rows of a label) and unnamed containers are left out.
fn dump(h: &Harness<'_, App>, t: &Path) -> String {
    let mut out = String::new();
    for node in h.root().children_recursive() {
        let n = node.accesskit_node();
        let role = n.role();
        let (label, value, description) = (n.label(), n.value(), n.description());
        if role == Role::TextRun
            || (role == Role::GenericContainer
                && label.is_none()
                && value.is_none()
                && description.is_none())
        {
            continue;
        }
        let _ = write!(out, "{role:?}");
        for (what, text) in [
            ("label", label),
            ("value", value),
            ("description", description),
        ] {
            if let Some(text) = text {
                let text = text
                    .replace(&t.display().to_string(), "<tmp>")
                    .replace('\\', "/");
                let _ = write!(out, " {what}={text:?}");
            }
        }
        if let Some(v) = n.numeric_value() {
            let _ = write!(out, " level={v}");
        }
        if n.is_disabled() {
            out.push_str(" disabled");
        }
        if let Some(toggled) = n.toggled() {
            let _ = write!(out, " toggled={toggled:?}");
        }
        if let Some(b) = n.bounding_box() {
            let _ = write!(
                out,
                " @{},{} {}x{}",
                b.x0.round(),
                b.y0.round(),
                (b.x1 - b.x0).round(),
                (b.y1 - b.y0).round()
            );
        }
        out.push('\n');
    }
    out
}

/// Compares the window `make` builds, at each size, with `ui/<name>`.
fn golden(name: &str, make: impl Fn(&Path, Vec2) -> Harness<'static, App>) {
    let mut out = String::new();
    for size in SIZES {
        let t = tempfile::tempdir().unwrap();
        let h = make(t.path(), size);
        let _ = write!(out, "== {}x{}\n{}", size.x, size.y, dump(&h, t.path()));
    }
    let host = if cfg!(windows) { "windows" } else { "linux" };
    rea_launcher::snapshot!(&format!("ui/{name}.{host}.txt"), out);
}

/// [`hub`] at `size`.
fn hub_at(t: &Path, size: Vec2) -> Harness<'static, App> {
    let lib = t.join("games");
    std::fs::create_dir_all(&lib).unwrap();
    let mut h = window_at(&paths_in(t), &[lib], size);
    wait(&mut h, "the scan", |a| a.entries.is_some());
    h
}

/// The hub with `games` found in the library, `slug` selected.
fn found(t: &Path, size: Vec2, games: &[&str], slug: &str) -> Harness<'static, App> {
    let mut h = hub_at(t, size);
    let mut entries: Vec<Entry> = games
        .iter()
        .map(|g| fake_entry(g, &t.join("games").join(g)))
        .collect();
    entries.push(Entry {
        dir: t.join("games/odd"),
        found: Found::Unknown {
            file: "id5.elf".into(),
            digest: "00ff".into(),
        },
    });
    h.state_mut().entries = Some(entries);
    select(&mut h, slug);
    h
}

#[test]
fn first_run() {
    golden("first-run", |t, size| window_at(&paths_in(t), &[], size));
}

#[test]
fn first_run_with_a_folder_added_and_a_wrong_bios() {
    golden("first-run-added", |t, size| {
        let mut h = window_at(&paths_in(t), &[], size);
        let lib = t.join("my games");
        std::fs::create_dir_all(&lib).unwrap();
        h.state_mut().folder = lib.display().to_string();
        click_button(&mut h, "Add folder");
        wait(&mut h, "the scan", |a| a.entries.is_some());
        h.state_mut().bios = "/no/such/naomi2.zip".into();
        click_button(&mut h, "Use BIOS");
        h
    });
}

#[test]
fn hub_while_looking() {
    golden("hub-looking", |t, size| {
        let mut h = hub_at(t, size);
        h.state_mut().entries = None;
        h.run_steps(3);
        h
    });
}

#[test]
fn hub_with_nothing_found() {
    golden("hub-none-found", hub_at);
}

#[test]
fn hub_with_games_found() {
    golden("hub-found", |t, size| {
        let mut h = found(t, size, &["id1", "id4", "id6", "zero", "ta5"], "id6");
        h.state_mut().config.naomi2_bios = Some(t.join("naomi2.zip"));
        h.run_steps(3);
        h
    });
}

#[test]
fn hub_naomi2_without_the_bios() {
    golden("hub-naomi2-no-bios", |t, size| {
        found(t, size, &["id1"], "id1")
    });
}

#[test]
fn hub_lindbergh_game_found() {
    golden("hub-lindbergh", |t, size| found(t, size, &["id5"], "id5"));
}

#[test]
fn hub_blocked_game() {
    golden("hub-blocked", |t, size| {
        let mut h = hub_at(t, size);
        let g = shown_as(&mut h, Support::Blocked("A reason it can't start."));
        h.state_mut().entries = Some(vec![fake_entry(g.slug, &t.join("games/x"))]);
        select(&mut h, g.slug);
        h
    });
}

#[test]
fn hub_unverified_game() {
    golden("hub-unverified", |t, size| {
        let mut h = hub_at(t, size);
        let g = shown_as(&mut h, Support::Unverified("Starts; no race yet."));
        h.state_mut().entries = Some(vec![fake_entry(g.slug, &t.join("games/x"))]);
        select(&mut h, g.slug);
        h
    });
}

fn settings_of_game(t: &Path, size: Vec2, slug: &str) -> Harness<'static, App> {
    let mut h = found(t, size, &[slug], slug);
    open_tab(&mut h, Tab::Settings);
    h
}

#[test]
fn settings_of_a_ringedge_game() {
    golden("settings-id6", |t, size| settings_of_game(t, size, "id6"));
}

#[test]
fn settings_of_a_naomi2_game() {
    golden("settings-id1", |t, size| settings_of_game(t, size, "id1"));
}

#[test]
fn settings_of_a_lindbergh_game() {
    golden("settings-id5", |t, size| settings_of_game(t, size, "id5"));
}

#[test]
fn settings_after_a_change() {
    golden("settings-saved", |t, size| {
        let mut h = settings_of_game(t, size, "zero");
        click_button(&mut h, "Game render size: next");
        h
    });
}

#[test]
fn setup_page() {
    golden("setup", |t, size| {
        let mut h = hub_at(t, size);
        open_tab(&mut h, Tab::Settings);
        click_button(&mut h, "Folders and BIOS");
        h
    });
}

#[test]
fn controls_without_a_controller() {
    golden("controls-none", |t, size| {
        let mut h = hub_at(t, size);
        open_tab(&mut h, Tab::Controls);
        h
    });
}

#[test]
fn controls_with_a_pad_and_a_wheel() {
    golden("controls-devices", |t, size| {
        let mut h = hub_at(t, size);
        open_tab(&mut h, Tab::Controls);
        let mut controls = rea_launcher::wheel::Controls {
            steer: -0.5,
            accel: 0.25,
            brake: 0.75,
            buttons: 0,
        };
        controls.press(Button::Start);
        h.state_mut().snap = Snapshot {
            devices: vec![
                DeviceInfo {
                    name: "Test Pad".into(),
                    kind: Kind::Pad,
                    haptic: false,
                },
                DeviceInfo {
                    name: "Test Wheel".into(),
                    kind: Kind::Wheel,
                    haptic: true,
                },
            ],
            driver: Some(1),
            controls,
            feeding: true,
            error: None,
        };
        h.run_steps(3);
        h
    });
}

#[test]
fn controls_with_an_error() {
    golden("controls-error", |t, size| {
        let mut h = hub_at(t, size);
        h.state_mut().snap.error = Some("controllers unavailable: no SDL".into());
        open_tab(&mut h, Tab::Controls);
        h
    });
}

/// The running overlay after `steps` reports from the session.
fn running(t: &Path, size: Vec2, steps: Vec<Progress>) -> Harness<'static, App> {
    let mut h = hub_at(t, size);
    let (tx, rx) = mpsc::channel();
    h.state_mut().running = Some(Running {
        game: ready(),
        setting_up: None,
        playing: false,
        progress: rx,
        stop: Arc::new(AtomicBool::new(false)),
    });
    for step in steps {
        tx.send(step).unwrap();
    }
    h.run_steps(3);
    h
}

#[test]
fn running_overlay_getting_ready() {
    golden("running-getting-ready", |t, size| running(t, size, vec![]));
}

#[test]
fn running_overlay_building_lindbergh() {
    golden("running-lindbergh-build", |t, size| {
        running(t, size, vec![Progress::SettingUp(Setup::Lindbergh)])
    });
}

#[test]
fn running_overlay_installing_visual_cpp() {
    golden("running-vc2008", |t, size| {
        running(t, size, vec![Progress::SettingUp(Setup::VisualCpp2008)])
    });
}

#[test]
fn running_overlay_playing() {
    golden("running-playing", |t, size| {
        running(t, size, vec![Progress::Playing])
    });
}

#[test]
fn a_game_that_stopped_with_an_error() {
    golden("message-stopped-with-error", |t, size| {
        let mut h = running(
            t,
            size,
            vec![
                Progress::Playing,
                Progress::Ended(Err("the loader crashed".into())),
            ],
        );
        h.run_steps(3);
        h
    });
}

#[test]
fn a_game_that_could_not_start() {
    golden("message-could-not-start", |t, size| {
        let mut h = found(t, size, &[ready().slug], ready().slug);
        click_button(&mut h, "Play");
        wait(&mut h, "the failure", |a| a.message.is_some());
        h
    });
}
