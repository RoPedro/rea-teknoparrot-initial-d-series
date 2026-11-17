//! The window over Initial D's real backend, headless: the games as
//! tiles, what each state says, every settings row reaching its file, the
//! setup page, the controllers, the session's stages, the fit at 720 and
//! 800 lines. The hub's own behaviour is tested in the SDK.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use initiald_core::catalog::by_slug;
use initiald_core::settings::Look;
use rea_launcher::gamescope::{Size, Upscaler};
use rea_launcher::hub::{Page, Tab};
use rea_launcher::library::Entry;
use rea_launcher::wheel::{Controls, Kind};
use rea_launcher_native::feeder::{DeviceInfo, Snapshot};
use rea_launcher_ui::egui::accesskit::Role;
use rea_launcher_ui::egui::{self, Vec2};
use rea_launcher_ui::hub::HubApp;
use rea_launcher_ui::hub::testing::{self as hub_testing, SIZE_720, SIZE_800, says};
use rea_launcher_ui::testing::kittest::NodeT as _;
use rea_launcher_ui::testing::{self, Harness, Queryable};

use super::*;

mod golden;

type H = Harness<'static, HubApp<Launcher>>;

/// A window over a launcher with `libraries` (and `bios`) in its
/// `launcher.cfg`, at `size`, without controllers.
fn window_at(t: &Path, libraries: &[PathBuf], bios: Option<PathBuf>, size: Vec2) -> H {
    let paths = crate::paths::in_dir(t);
    let mut config = HubConfig::new(&FIELDS);
    for l in libraries {
        config.add_library(l);
    }
    config.paths[0] = bios;
    if config != HubConfig::new(&FIELDS) {
        config.save(&paths.config_file(), &FIELDS).unwrap();
    }
    hub_testing::harness(spec(paths.clone()), Launcher::new(paths, None), size)
}

/// A window over an empty library folder, once its scan is done.
fn hub_at(t: &Path, size: Vec2) -> H {
    let lib = t.join("games");
    std::fs::create_dir_all(&lib).unwrap();
    let mut h = window_at(t, &[lib], None, size);
    hub_testing::wait(&mut h, "the scan", |a| a.library().is_some());
    h
}

fn hub(t: &Path) -> H {
    hub_at(t, SIZE_720)
}

fn index(slug: &str) -> usize {
    GAMES.iter().position(|g| g.slug == slug).unwrap()
}

/// A verified Windows game (the catalog changes as games are verified,
/// so the tests ask for a kind, not a name).
fn ready() -> &'static Game {
    GAMES
        .iter()
        .find(|g| g.support == Support::Ready && matches!(g.route, Route::Wal { .. }))
        .expect("a verified Windows game")
}

/// A catalog game shown with `support`.
fn shown_as(h: &mut H, support: Support) -> &'static Game {
    let g = ready();
    h.state_mut().backend_mut().supports[index(g.slug)] = support;
    g
}

fn fake_entry(slug: &str, dir: &Path) -> Entry<Game> {
    let game = by_slug(slug).unwrap();
    Entry {
        dir: dir.to_path_buf(),
        file: dir.join(game.main_file),
        found: library::Found::Known {
            game,
            edition: &game.editions[0],
        },
    }
}

/// The library holding `entries`, as a scan would return it.
fn set_found(h: &mut H, entries: Vec<Entry<Game>>) {
    h.state_mut().set_library(Scan {
        entries,
        problems: Vec::new(),
    });
    h.run_steps(3);
}

fn tile<'h>(h: &'h H, slug: &str) -> testing::Node<'h> {
    h.get_by_role_and_label(Role::RadioButton, by_slug(slug).unwrap().title)
}

fn select(h: &mut H, slug: &str) {
    tile(h, slug).click();
    h.run_steps(3);
    assert_eq!(GAMES[h.state().model().selected].slug, slug);
}

fn click_button(h: &mut H, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(3);
}

fn open_tab(h: &mut H, tab: Tab) {
    h.get_by_role_and_label(Role::Tab, tab.label()).click();
    h.run_steps(3);
}

fn type_into(h: &mut H, field: &str, text: &str) {
    let f = h.get_by_role_and_label(Role::TextInput, field);
    f.click();
    f.type_text(text);
    h.run_steps(3);
}

fn settings_of(t: &Path, slug: &str) -> (Settings, String) {
    let text = std::fs::read_to_string(crate::paths::in_dir(t).settings(slug)).unwrap_or_default();
    (Settings::parse(&text).0, text)
}

/// One step of a scripted session.
enum Step {
    SettingUp(Setup),
    Playing,
    End(Result<(), String>),
}

/// The next Play's session: it does what `rx` says, step by step, and
/// ends when the window asks it to stop.
fn scripted(h: &mut H, rx: mpsc::Receiver<Step>) {
    h.state_mut().backend_mut().next = Some(Box::new(move |r: &Reporter| {
        loop {
            match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(Step::SettingUp(s)) => {
                    let (title, body) = stage(s);
                    r.setting_up(title, body);
                }
                Ok(Step::Playing) => r.playing(),
                Ok(Step::End(result)) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) if !r.stop_requested() => {}
                Err(_) => return Ok(()),
            }
        }
    }));
}

/// The pad and the wheel the controls tests show.
fn two_devices() -> Snapshot {
    let mut controls = Controls {
        steer: -0.5,
        accel: 0.25,
        brake: 0.75,
        buttons: 0,
    };
    controls.press(Button::Start);
    Snapshot {
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
        wheel: None,
        controls,
        feeding: true,
        error: None,
        nav: 0,
    }
}

// ------------------------------------------------------------ the look

#[test]
fn the_window_is_not_named_like_a_game() {
    assert_eq!(THEME.title, "Initial D Launcher");
    assert!(GAMES.iter().all(|g| g.title != THEME.title));
}

#[test]
fn the_palette_meets_wcag_aa() {
    let problems = THEME.contrast_problems();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn every_option_has_a_settings_row_and_a_flag() {
    let rows: Vec<&str> = settings::OPTIONS
        .iter()
        .filter(|o| !o.launcher_only)
        .map(|o| o.key)
        .collect();
    rea_launcher::cfg::spec::assert_parity(settings::OPTIONS, &rows);
}

// ------------------------------------------------------------ hub

#[test]
fn the_hub_has_a_tile_for_each_of_the_eleven_games() {
    let t = tempfile::tempdir().unwrap();
    let h = hub(t.path());
    assert_eq!(h.get_all_by_role(Role::RadioButton).count(), 11);
    for g in GAMES {
        let about = testing::description(&tile(&h, g.slug)).unwrap();
        assert!(about.contains(&g.year.to_string()), "{about}");
        assert!(about.contains(g.board), "{about}");
    }
    assert!(says(&h, "Your games · 0 of 11 found"));
}

#[test]
fn a_found_game_shows_its_edition_and_folder() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let g = ready();
    let dir = t.path().join("games/ready");
    set_found(&mut h, vec![fake_entry(g.slug, &dir)]);
    select(&mut h, g.slug);
    assert!(says(
        &h,
        &format!("{} · {} · {}", g.board, g.year, g.editions[0].label)
    ));
    assert!(says(
        &h,
        &format!("Folder: {}", rea_launcher::hub::display_path(&dir))
    ));
    assert!(!testing::disabled(
        &h.get_by_role_and_label(Role::Button, "Play")
    ));
    assert!(says(&h, "Your games · 1 of 11 found"));
}

#[test]
fn a_blocked_game_cannot_be_played_and_says_why() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let why = "Not yet: it crashes.";
    let g = shown_as(&mut h, Support::Blocked(why));
    set_found(&mut h, vec![fake_entry(g.slug, &t.path().join("games/x"))]);
    select(&mut h, g.slug);
    let play = h.get_by_role_and_label(Role::Button, "Play");
    assert!(testing::disabled(&play));
    assert!(says(&h, why));
    click_button(&mut h, "Play");
    assert!(!h.state().is_running() && h.state().message().is_none());
}

#[test]
fn a_naomi2_game_without_the_bios_says_so_on_its_page() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    set_found(&mut h, vec![fake_entry("id1", &t.path().join("games/n"))]);
    select(&mut h, "id1");
    assert!(says(&h, "Needs the NAOMI 2 BIOS: add it in Settings."));
    select(&mut h, ready().slug);
    assert!(!says(&h, "Needs the NAOMI 2 BIOS"));
}

#[test]
fn play_reports_why_it_could_not_start_and_logs_it() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let g = ready();
    set_found(&mut h, vec![fake_entry(g.slug, &t.path().join("games/r"))]);
    select(&mut h, g.slug);
    click_button(&mut h, "Play");
    // the launcher's components are not in the test folder
    hub_testing::wait(&mut h, "the failure", |a| a.message().is_some());
    assert!(says(&h, &format!("{} couldn't start", g.title)));
    assert!(says(&h, "needs launcher components that are missing"));
    let log = std::fs::read_to_string(crate::paths::in_dir(t.path()).log()).unwrap();
    assert!(
        log.contains("needs launcher components that are missing"),
        "{log}"
    );
}

#[test]
fn the_first_starts_setup_stages_are_shown_and_stop_ends_the_session() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let g = ready();
    set_found(&mut h, vec![fake_entry(g.slug, &t.path().join("games/r"))]);
    select(&mut h, g.slug);
    let (tx, rx) = mpsc::channel();
    scripted(&mut h, rx);
    click_button(&mut h, "Play");
    assert!(says(&h, &format!("Getting {} ready", g.title)));
    tx.send(Step::SettingUp(Setup::Lindbergh)).unwrap();
    h.run_steps(5);
    assert!(says(&h, "Building lindbergh-loader"));
    tx.send(Step::SettingUp(Setup::VisualCpp2008)).unwrap();
    h.run_steps(5);
    assert!(says(&h, "Installing Microsoft Visual C++ 2008"));
    assert!(says(&h, "Windows asks for your permission first"));
    tx.send(Step::Playing).unwrap();
    h.run_steps(5);
    assert!(says(&h, &format!("{} is running", g.title)));
    click_button(&mut h, "Stop");
    hub_testing::wait(&mut h, "the end", |a| !a.is_running());
    assert!(h.state().message().is_none());
}

#[test]
fn closing_the_window_stops_the_game_first() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let g = ready();
    set_found(&mut h, vec![fake_entry(g.slug, &t.path().join("games/r"))]);
    select(&mut h, g.slug);
    let (tx, rx) = mpsc::channel();
    scripted(&mut h, rx);
    click_button(&mut h, "Play");
    tx.send(Step::Playing).unwrap();
    h.run_steps(5);
    hub_testing::request_close(&mut h);
    h.run_steps(1);
    assert!(hub_testing::commands(&h).contains(&egui::ViewportCommand::CancelClose));
    hub_testing::wait(&mut h, "the stop", |a| !a.is_running());
    drop(tx);
}

#[test]
fn the_keyboard_moves_between_tiles_and_pages() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    for _ in 0..2 {
        h.key_press(egui::Key::ArrowRight);
        h.run_steps(2);
    }
    assert_eq!(h.state().model().selected, 2);
    h.key_press(egui::Key::PageDown);
    h.run_steps(3);
    assert_eq!(h.state().model().page, Page::Controls);
    h.key_press(egui::Key::PageDown);
    h.run_steps(3);
    assert!(says(&h, "Initial D Arcade Stage Ver. 3 settings"));
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(h.state().model().page, Page::Hub);
}

#[test]
fn the_pad_moves_between_tiles_and_y_opens_the_settings() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let press = |h: &mut H, bits: u16| {
        h.state_mut().backend_mut().snapshot.nav = bits;
        h.run_steps(2);
        h.state_mut().backend_mut().snapshot.nav = 0;
        h.run_steps(2);
    };
    press(&mut h, rea_launcher::pad::DPAD_RIGHT);
    assert_eq!(h.state().model().selected, 1);
    press(&mut h, rea_launcher::pad::Y);
    assert_eq!(h.state().model().page, Page::Settings);
    press(&mut h, rea_launcher::pad::B);
    assert_eq!(h.state().model().page, Page::Hub);
}

// ------------------------------------------------------------ settings

/// What each row's arrow does to the settings.
type Change = Box<dyn Fn(&mut Settings)>;

#[test]
#[cfg_attr(windows, ignore = "the gamescope rows are off on Windows")]
fn every_settings_row_reaches_its_file() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let file = crate::paths::in_dir(t.path()).settings(GAMES[0].slug);
    let notes = "# my notes\nfuture_key = 7\ncrt = lottes\n";
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, notes).unwrap();
    open_tab(&mut h, Tab::Settings);
    assert!(!says(&h, "CRT"), "no row for the CRT look");
    let mut expect = Settings::parse(notes).0;
    let steps: [(&str, &str, Change); 6] = [
        (
            "Game render size",
            "next",
            Box::new(|s| s.render = Some(Size::new(1920, 1080))),
        ),
        (
            "Output size",
            "previous",
            Box::new(|s| s.output = Size::new(2560, 1440)),
        ),
        ("Upscaler", "next", Box::new(|s| s.upscaler = Upscaler::Nis)),
        ("Sharpness", "next", Box::new(|s| s.sharpness = 6)),
        ("Fullscreen", "next", Box::new(|s| s.fullscreen = false)),
        ("Dead zone", "next", Box::new(|s| s.deadzone = 0.06)),
    ];
    for (label, way, change) in &steps {
        click_button(&mut h, &format!("{label}: {way}"));
        change(&mut expect);
        let (saved, text) = settings_of(t.path(), GAMES[0].slug);
        assert_eq!(saved, expect, "{label}\n{text}");
        assert!(
            text.starts_with(notes),
            "{label}: the player's lines stay\n{text}"
        );
        assert!(says(&h, "Saved."), "{label}");
    }
    let (_, text) = settings_of(t.path(), GAMES[0].slug);
    assert!(text.ends_with(
        "render = 1920x1080\noutput = 2560x1440\nupscaler = nis\nsharpness = 6\n\
         fullscreen = false\ndeadzone = 0.06\n"
    ));
}

#[test]
#[cfg(windows)]
fn on_windows_the_gamescope_rows_are_off_and_say_why() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    open_tab(&mut h, Tab::Settings);
    for label in ["Output size", "Upscaler", "Sharpness", "Fullscreen"] {
        let row = h.get_by_role_and_label(Role::Button, label);
        assert!(testing::disabled(&row), "{label}");
        assert!(testing::description(&row).unwrap().contains("gamescope"));
    }
    for label in ["Game render size", "Dead zone"] {
        assert!(!testing::disabled(
            &h.get_by_role_and_label(Role::Button, label)
        ));
    }
}

#[test]
#[cfg_attr(windows, ignore = "the gamescope rows are off on Windows")]
fn sharpness_goes_off_without_fsr_or_nis() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    open_tab(&mut h, Tab::Settings);
    for _ in 0..2 {
        click_button(&mut h, "Upscaler");
    }
    let sharp = h.get_by_role_and_label(Role::Button, "Sharpness");
    assert!(testing::disabled(&sharp));
    assert!(testing::description(&sharp).unwrap().contains("FSR 1"));
}

#[test]
fn settings_belong_to_the_selected_game() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    let g = GAMES.last().unwrap();
    select(&mut h, g.slug);
    open_tab(&mut h, Tab::Settings);
    assert!(says(&h, &format!("{} settings", g.title)));
    click_button(&mut h, "Dead zone: next");
    assert_eq!(settings_of(t.path(), g.slug).0.deadzone, 0.06);
    assert!(
        !crate::paths::in_dir(t.path())
            .settings(GAMES[0].slug)
            .exists()
    );
    // a game seen for the first time gets the recommended look, saved
    let look = h.get_by_role_and_label(Role::Button, "Look");
    assert_eq!(testing::value(&look).as_deref(), Some("Recommended"));
    assert_eq!(
        settings_of(t.path(), g.slug).0.look,
        Some(Look::Recommended)
    );
}

#[test]
#[cfg_attr(windows, ignore = "the gamescope rows are off on Windows")]
fn the_look_row_sets_the_picture_and_a_hand_change_makes_it_custom() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    open_tab(&mut h, Tab::Settings);
    let value = |h: &H, label: &str| testing::value(&h.get_by_role_and_label(Role::Button, label));
    click_button(&mut h, "Look: previous");
    assert_eq!(value(&h, "Look").as_deref(), Some("Original"));
    let (s, text) = settings_of(t.path(), GAMES[0].slug);
    assert_eq!(
        (s.look, s.render, s.upscaler),
        (Some(Look::Original), None, Upscaler::Linear)
    );
    assert!(text.contains("\nlook = original\n"), "{text}");
    click_button(&mut h, "Upscaler: next");
    assert_eq!(value(&h, "Look").as_deref(), Some("Custom"));
    let (s, text) = settings_of(t.path(), GAMES[0].slug);
    assert_eq!(s.look, None);
    assert!(
        !text.contains("\nlook ="),
        "a look given up is cleared:\n{text}"
    );
}

// ------------------------------------------------------------ setup

#[test]
fn the_setup_page_adds_a_folder_and_the_bios_and_keeps_the_players_lines() {
    let t = tempfile::tempdir().unwrap();
    let paths = crate::paths::in_dir(t.path());
    std::fs::create_dir_all(&paths.data).unwrap();
    std::fs::write(paths.config_file(), "# mine\nfuture = 1\n").unwrap();
    let mut h = window_at(t.path(), &[], None, SIZE_720);
    assert_eq!(h.state().model().page, Page::Setup);
    let lib = t.path().join("my games");
    std::fs::create_dir_all(&lib).unwrap();
    type_into(&mut h, "Game folder", &lib.display().to_string());
    click_button(&mut h, "Add folder");
    click_button(&mut h, "Use BIOS");
    assert!(says(&h, "Type the path of naomi2.zip first."));
    let bios = t.path().join("naomi2.zip");
    std::fs::write(&bios, b"zip").unwrap();
    type_into(&mut h, "NAOMI 2 BIOS file", &bios.display().to_string());
    click_button(&mut h, "Use BIOS");
    assert!(says(&h, "NAOMI 2 BIOS saved."));
    assert_eq!(
        std::fs::read_to_string(paths.config_file()).unwrap(),
        format!(
            "# mine\nfuture = 1\nlibrary = {}\nnaomi2_bios = {}\n",
            lib.display(),
            bios.display()
        )
    );
    hub_testing::wait(&mut h, "the scan", |a| a.library().is_some());
    click_button(&mut h, "Done");
    assert_eq!(h.state().model().page, Page::Hub);
}

// ------------------------------------------------------------ controls

#[test]
fn the_controls_page_shows_the_wheel_and_live_steering_and_pedals() {
    let t = tempfile::tempdir().unwrap();
    let mut h = hub(t.path());
    open_tab(&mut h, Tab::Controls);
    assert!(says(&h, "No controller found."));
    h.state_mut().backend_mut().snapshot = two_devices();
    h.run_steps(3);
    assert!(says(&h, "Wheel: Test Wheel (drives the car)"));
    assert!(says(&h, "Pad: Test Pad"));
    let level = |h: &H, name: &str| {
        let m = h.get_by_role_and_label(Role::Meter, name);
        m.accesskit_node().numeric_value().unwrap()
    };
    assert_eq!(level(&h, "Steering"), -50.0);
    assert_eq!(level(&h, "Brake"), 75.0);
    assert!(says(&h, "Start: pressed"));
    assert!(says(&h, "Insert coin: released"));
    // the pad layout the launcher gives a pad
    assert!(says(&h, "Gas: RT"));
    assert!(says(&h, "View change: Y"));
}

// ------------------------------------------------------------ layout

#[test]
fn every_control_fits_the_window_on_every_page() {
    for size in [SIZE_720, SIZE_800] {
        let t = tempfile::tempdir().unwrap();
        let mut h = hub_at(t.path(), size);
        h.state_mut().backend_mut().snapshot = two_devices();
        let check = |h: &mut H, page: &str| {
            h.run_steps(3);
            let misfits = hub_testing::misfits(h);
            assert!(misfits.is_empty(), "{size:?} {page}: {misfits:#?}");
        };
        check(&mut h, "hub");
        open_tab(&mut h, Tab::Controls);
        check(&mut h, "controls");
        open_tab(&mut h, Tab::Settings);
        check(&mut h, "settings");
        click_button(&mut h, "Folders and BIOS");
        check(&mut h, "setup");
    }
}
