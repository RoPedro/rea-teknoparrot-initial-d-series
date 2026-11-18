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

/// One line per node a player can perceive, in tree order: text runs (the
/// rows of a label) and unnamed containers are left out. (The format of
/// the launcher's goldens before the SDK hub, so its diffs read alone.)
fn dump(h: &H, t: &Path) -> String {
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
fn golden(name: &str, make: impl Fn(&Path, Vec2) -> H) {
    let mut out = String::new();
    for size in [SIZE_720, SIZE_800] {
        // a folder whose path has the same width every run: a message
        // naming it wraps the same way (a random temp name moved a button)
        let t = std::env::temp_dir()
            .join("rea-initiald-golden")
            .join(format!("{name}-{}", size.y));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).unwrap();
        let h = make(&t, size);
        let _ = write!(out, "== {}x{}\n{}", size.x, size.y, dump(&h, &t));
        drop(h);
        let _ = std::fs::remove_dir_all(&t);
    }
    let host = if cfg!(windows) { "windows" } else { "linux" };
    rea_launcher::snapshot!(&format!("ui/{name}.{host}.txt"), out);
}

/// The hub with `games` found in the library (and an unknown build),
/// `slug` selected.
fn found(t: &Path, size: Vec2, games: &[&str], slug: &str) -> H {
    let mut h = hub_at(t, size);
    let mut entries: Vec<Entry<Game>> = games
        .iter()
        .map(|g| fake_entry(g, &t.join("games").join(g)))
        .collect();
    entries.push(Entry {
        dir: t.join("games/odd"),
        file: t.join("games/odd/id5.elf"),
        found: library::Found::Unknown {
            file: "id5.elf".into(),
            digest: "00ff".into(),
        },
    });
    set_found(&mut h, entries);
    select(&mut h, slug);
    h
}

#[test]
fn first_run() {
    golden("first-run", |t, size| window_at(t, &[], None, size));
}

#[test]
fn first_run_with_a_folder_added_and_a_wrong_bios() {
    golden("first-run-added", |t, size| {
        let mut h = window_at(t, &[], None, size);
        let lib = t.join("my games");
        std::fs::create_dir_all(&lib).unwrap();
        type_into(&mut h, "Game folder", &lib.display().to_string());
        click_button(&mut h, "Add folder");
        hub_testing::wait(&mut h, "the scan", |a| a.library().is_some());
        type_into(&mut h, "NAOMI 2 BIOS file", "/no/such/naomi2.zip");
        click_button(&mut h, "Use BIOS");
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
        let lib = t.join("games");
        std::fs::create_dir_all(&lib).unwrap();
        let mut h = window_at(t, &[lib], Some(t.join("naomi2.zip")), size);
        hub_testing::wait(&mut h, "the scan", |a| a.library().is_some());
        let entries = ["id1", "id4", "id6", "zero", "ta5"]
            .iter()
            .map(|g| fake_entry(g, &t.join("games").join(g)))
            .collect();
        set_found(&mut h, entries);
        select(&mut h, "id6");
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
        set_found(&mut h, vec![fake_entry(g.slug, &t.join("games/x"))]);
        select(&mut h, g.slug);
        h
    });
}

#[test]
fn hub_unverified_game() {
    golden("hub-unverified", |t, size| {
        let mut h = hub_at(t, size);
        let g = shown_as(&mut h, Support::Unverified("Starts; no race yet."));
        set_found(&mut h, vec![fake_entry(g.slug, &t.join("games/x"))]);
        select(&mut h, g.slug);
        h
    });
}

fn settings_of_game(t: &Path, size: Vec2, slug: &str) -> H {
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
        h.state_mut().backend_mut().snapshot = two_devices();
        h.run_steps(3);
        h
    });
}

#[test]
fn controls_with_an_error() {
    golden("controls-error", |t, size| {
        let mut h = hub_at(t, size);
        h.state_mut().backend_mut().snapshot.error = Some("controllers unavailable: no SDL".into());
        open_tab(&mut h, Tab::Controls);
        h
    });
}

/// The running overlay of a ready game after the session's `steps`.
fn running(t: &Path, size: Vec2, steps: Vec<Step>) -> H {
    let g = ready();
    let mut h = found(t, size, &[g.slug], g.slug);
    let (tx, rx) = mpsc::channel();
    scripted(&mut h, rx);
    click_button(&mut h, "Play");
    for step in steps {
        tx.send(step).unwrap();
        // the session thread takes it within its 10 ms poll
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.run_steps(3);
    }
    // the test's end lets the session go
    std::mem::forget(tx);
    h
}

#[test]
fn running_overlay_getting_ready() {
    golden("running-getting-ready", |t, size| running(t, size, vec![]));
}

#[test]
fn running_overlay_building_lindbergh() {
    golden("running-lindbergh-build", |t, size| {
        running(t, size, vec![Step::SettingUp(Setup::Lindbergh)])
    });
}

#[test]
fn running_overlay_installing_visual_cpp() {
    golden("running-vc2008", |t, size| {
        running(t, size, vec![Step::SettingUp(Setup::VisualCpp2008)])
    });
}

#[test]
fn running_overlay_playing() {
    golden("running-playing", |t, size| {
        running(t, size, vec![Step::Playing])
    });
}

#[test]
fn a_game_that_stopped_with_an_error() {
    golden("message-stopped-with-error", |t, size| {
        let mut h = running(
            t,
            size,
            vec![Step::Playing, Step::End(Err("the loader crashed".into()))],
        );
        hub_testing::wait(&mut h, "the end", |a| !a.is_running());
        h
    });
}

#[test]
fn a_game_that_could_not_start() {
    golden("message-could-not-start", |t, size| {
        let mut h = found(t, size, &[ready().slug], ready().slug);
        click_button(&mut h, "Play");
        hub_testing::wait(&mut h, "the failure", |a| a.message().is_some());
        h
    });
}
