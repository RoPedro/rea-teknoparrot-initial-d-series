//! The window: the SDK's hub look (`rea_launcher_ui::hub`) over Initial D's
//! games. What stays here is the launcher's: its colours, its games as
//! hub tiles, the NAOMI 2 BIOS field, and [`Launcher`], the
//! [`HubBackend`] that scans the library, loads a game's settings,
//! prepares its session (the lindbergh-loader build and Visual C++ 2008
//! setup stages included) and shows the controllers.

use std::path::PathBuf;
use std::time::Duration;

use initiald_core::catalog::{GAMES, Game, Route};
use initiald_core::input::{Button, default_pad_map};
use initiald_core::settings::{self, Settings};
use rea_launcher::catalog::{Host, Support};
use rea_launcher::cfg::OptionSpec;
use rea_launcher::hub::{HubConfig, HubPaths, Input};
use rea_launcher::library::{self, HashCache, Library, Scan};
use rea_launcher::log::Log;
use rea_launcher::pad::PadNav;
use rea_launcher::wheel::{Action, Control};
use rea_launcher_native::feeder::{self, InputService};
use rea_launcher_ui::egui::Color32;
use rea_launcher_ui::hub::{
    self, ButtonView, ControlsView, Device, Found, HubBackend, HubGame, HubLayout, HubSpec,
    HubTheme, HubWords, Note, Reporter, Scanner, Session,
};

use crate::form::Form;
use crate::paths::FIELDS;
use crate::session::{self, Display, Feed, Setup};

/// A mountain pass at night: dark asphalt, headlight amber and the
/// blue-and-white of Japanese mountain road signs.
pub const THEME: HubTheme = HubTheme {
    base: Color32::from_rgb(0x0B, 0x12, 0x1A),
    panel: Color32::from_rgb(0x14, 0x1E, 0x2A),
    line: Color32::from_rgb(0x6B, 0x7A, 0x8C),
    lamp: Color32::from_rgb(0xFF, 0xC8, 0x57),
    lamp_hover: Color32::from_rgb(0xFF, 0xD7, 0x84),
    sign: Color32::from_rgb(0x1D, 0x4F, 0x8F),
    ink: Color32::from_rgb(0xF4, 0xF7, 0xF2),
    muted: Color32::from_rgb(0x8A, 0x97, 0xA6),
    signal: Color32::from_rgb(0xB3, 0x26, 0x1E),
    error: Color32::from_rgb(0xFF, 0x8A, 0x80),
    disabled: Color32::from_rgb(0x5E, 0x6B, 0x7A),
    focus_fill: Color32::from_rgb(0x1F, 0x2C, 0x3B),
    scrim: 240,
    brand: "ID",
    title: "Initial D Launcher",
    app_id: "rea-initiald",
};

pub const WORDS: HubWords = HubWords {
    setup_intro: "Type the path of a folder that holds your Initial D game folders and add it. \
         The launcher looks inside for each game and never changes what's there.",
    folders_button: "Folders and BIOS",
    ..HubWords::DEFAULT
};

/// What the window is, besides its backend.
pub fn spec(paths: HubPaths) -> HubSpec {
    HubSpec {
        theme: THEME,
        layout: HubLayout::DEFAULT,
        words: WORDS,
        fields: &FIELDS,
        paths,
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Opens the hub and runs it until it is closed.
pub fn run(paths: HubPaths) -> Result<(), String> {
    let launcher = Launcher::new(paths.clone(), Some(crate::input::start()));
    hub::run(spec(paths), launcher)
}

/// Initial D's games as the hub's tiles.
fn hub_games() -> Vec<HubGame> {
    GAMES
        .iter()
        .map(|g| HubGame {
            slug: g.slug,
            title: g.title,
            short: g.short,
            board: g.board,
            year: g.year,
        })
        .collect()
}

/// The launcher behind the hub.
pub struct Launcher {
    paths: HubPaths,
    log: Log,
    games: Vec<HubGame>,
    /// Each game's support on this system (the tests set others).
    supports: Vec<Support>,
    /// Digests shared by every scan (a rescan reuses a running one's).
    cache: HashCache,
    /// Owns SDL; `None` in the headless tests.
    input: Option<InputService>,
    /// The pad's menu buttons, held between frames.
    pad: PadNav,
    /// The controllers the tests show instead of the input thread's.
    #[cfg(test)]
    snapshot: feeder::Snapshot,
    /// The session the tests start instead of a game's.
    #[cfg(test)]
    next: Option<Session>,
}

impl Launcher {
    pub fn new(paths: HubPaths, input: Option<InputService>) -> Launcher {
        Launcher {
            log: Log::new(paths.log()),
            paths,
            games: hub_games(),
            supports: GAMES
                .iter()
                .map(|g| g.support_on(Host::current()))
                .collect(),
            cache: HashCache::default(),
            input,
            pad: PadNav::with_repeat(Duration::from_millis(400), Duration::from_millis(120)),
            #[cfg(test)]
            snapshot: feeder::Snapshot::default(),
            #[cfg(test)]
            next: None,
        }
    }

    fn snapshot(&self) -> feeder::Snapshot {
        #[cfg(test)]
        return self.snapshot.clone();
        #[cfg(not(test))]
        self.input
            .as_ref()
            .map(InputService::snapshot)
            .unwrap_or_default()
    }
}

/// What the running overlay says while a first start sets something up.
fn stage(setup: Setup) -> (&'static str, &'static str) {
    match setup {
        Setup::Lindbergh => (
            "Building lindbergh-loader…",
            "Initial D 4 and 5 run in lindbergh-loader, built once on your computer \
             (its licence forbids shipping it). This takes a few minutes.",
        ),
        Setup::VisualCpp2008 => (
            "Installing Microsoft Visual C++ 2008…",
            "Initial D 6, 7 and 8 need this Microsoft runtime, installed once. \
             Windows asks for your permission first.",
        ),
    }
}

impl HubBackend for Launcher {
    type Library = Scan<Game>;
    type Form = Form;

    fn games(&self) -> &[HubGame] {
        &self.games
    }

    fn options(&self) -> &'static [OptionSpec] {
        settings::OPTIONS
    }

    fn support(&self, game: usize) -> Support {
        self.supports[game]
    }

    fn note(&self, game: usize, config: &HubConfig) -> Option<Note> {
        (matches!(GAMES[game].route, Route::Flycast { .. }) && config.paths[0].is_none())
            .then(|| Note::error("Needs the NAOMI 2 BIOS: add it in Settings."))
    }

    fn scanner(&self) -> Scanner<Scan<Game>> {
        let library = Library::new(GAMES).with_cache(self.cache.clone());
        Box::new(move |roots| library.scan(roots))
    }

    fn found(scan: &Scan<Game>, game: usize) -> Option<Found> {
        let entry = *scan.by_game().get(&GAMES[game].id)?;
        let library::Found::Known { edition, .. } = entry.found else {
            return None;
        };
        Some(Found {
            dir: entry.dir.clone(),
            edition: Some(edition.label.into()),
        })
    }

    fn unknown(scan: &Scan<Game>) -> Vec<String> {
        let unknown = scan.entries.iter().filter_map(|e| match &e.found {
            library::Found::Unknown { file, digest } => Some(format!(
                "unknown build {file} {digest} in {}",
                e.dir.display()
            )),
            library::Found::Known { .. } => None,
        });
        let problems = scan.problems.iter().map(|p| format!("warning: {p}"));
        unknown.chain(problems).collect()
    }

    fn form(&mut self, game: usize, _screen: Option<(u32, u32)>) -> Form {
        let g = &GAMES[game];
        let file = self.paths.settings(g.slug);
        let settings = match Settings::load(&file) {
            Ok((s, problems)) => {
                for p in problems {
                    self.log.line(&format!("{}: {p}", file.display()));
                }
                s
            }
            // shown with the defaults; a save refuses to write over it
            Err(e) => {
                self.log.error(&format!("{}: {e}", file.display()));
                Settings::default()
            }
        };
        Form::new(settings, g.native)
    }

    fn session(
        &mut self,
        game: usize,
        library: &Scan<Game>,
        config: &HubConfig,
        _screen: Option<(u32, u32)>,
    ) -> Result<Session, String> {
        #[cfg(test)]
        if let Some(next) = self.next.take() {
            return Ok(next);
        }
        let g: &'static Game = &GAMES[game];
        let entry = (*library
            .by_game()
            .get(&g.id)
            .ok_or("it is not in your library folders")?)
        .clone();
        let file = self.paths.settings(g.slug);
        let (settings, _) =
            Settings::load(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let feed = match (g.route, &self.input) {
            (Route::Wal { .. }, Some(input)) => Feed::Devices {
                tx: input.sender(),
                deadzone: settings.deadzone,
            },
            _ => Feed::None,
        };
        let (paths, config) = (self.paths.clone(), config.clone());
        Ok(Box::new(move |r: &Reporter| {
            let fail = |e: anyhow::Error| format!("{e:#}");
            if let Some(setup) = session::setup_needed(&paths, g) {
                let (title, body) = stage(setup);
                r.setting_up(title, body);
                session::set_up(&paths, setup).map_err(fail)?;
            }
            let prepared = session::prepare(&paths, g, &entry, &settings, &config, Display::Screen)
                .map_err(fail)?;
            r.playing();
            let out = session::run(&prepared, &feed, &r.stop_flag()).map_err(fail)?;
            session::verdict(&prepared, &out)
        }))
    }

    fn game_log(&self, game: usize) -> Option<PathBuf> {
        Some(session::log_path(&self.paths.runtime(GAMES[game].slug)))
    }

    fn ended(&mut self) {
        if let Some(input) = &self.input {
            let _ = input.sender().send(feeder::Command::Detach);
        }
    }

    fn controls(&self) -> ControlsView {
        controls_view(&self.snapshot())
    }

    fn pad(&mut self) -> Vec<Input> {
        let bits = self.snapshot().nav;
        Input::from_pad(&mut self.pad, bits)
    }
}

/// The Controls page and the top bar from the input thread's snapshot.
fn controls_view(snap: &feeder::Snapshot) -> ControlsView {
    let map = default_pad_map();
    let mut layout = vec![
        ("Steer".to_string(), "Left stick".to_string()),
        ("Gas".into(), map.describe(Action::Accelerate)),
        ("Brake".into(), map.describe(Action::Brake)),
    ];
    layout.extend(Button::ALL.iter().filter_map(|b| {
        let keys = map.describe(Action::Press(*b));
        (keys != "-").then(|| (b.label().to_string(), keys))
    }));
    let c = &snap.controls;
    ControlsView {
        error: snap.error.clone(),
        devices: snap
            .devices
            .iter()
            .map(|d| Device {
                name: d.name.clone(),
                kind: d.kind,
            })
            .collect(),
        driver: snap.driver,
        layout,
        layout_note: "A wheel steers with its wheel and pedals, and its paddles shift.".into(),
        steer: c.steer,
        accel: c.accel,
        brake: c.brake,
        buttons: Button::ALL
            .iter()
            .map(|b| ButtonView {
                key: b.key(),
                label: b.label().into(),
                pressed: c.is_pressed(*b),
            })
            .collect(),
        feeding: snap.feeding,
    }
}

#[cfg(test)]
mod tests;
