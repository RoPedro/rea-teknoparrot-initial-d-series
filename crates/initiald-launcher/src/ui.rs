//! The window: the SDK's hub look (`rea_launcher_ui::hub`) over Initial D's
//! games. What stays here is the launcher's: its colours, its games as
//! hub tiles, the NAOMI 2 BIOS field, and [`Launcher`], the
//! [`HubBackend`] that scans the library, loads a game's settings,
//! prepares its session (the lindbergh-loader build and Visual C++ 2008
//! setup stages included) and shows the controllers.

use std::path::PathBuf;
use std::time::Duration;

use initiald_core::catalog::{GAMES, Game, Route};
use initiald_core::input::Button;
use initiald_core::settings::{self, Settings};
use rea_launcher::catalog::{Host, Support};
use rea_launcher::cfg::OptionSpec;
use rea_launcher::hub::bindings::Bindings;
use rea_launcher::hub::{HubConfig, HubPaths, Input};
use rea_launcher::library::{self, HashCache, Library, Scan};
use rea_launcher::log::Log;
use rea_launcher::pad::PadNav;
use rea_launcher::wheel::{Control, PadInput};
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
    let log = Log::new(paths.log());
    let bindings = crate::input::bindings(&paths, |w| log.warn(w));
    let mut launcher = Launcher::new(paths.clone(), Some(crate::input::start(&bindings)));
    launcher.bindings = bindings;
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
    /// The player's pad layout (`controls.cfg`).
    bindings: Bindings<Button>,
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
            bindings: Bindings::new(initiald_core::input::default_pad_map()),
            #[cfg(test)]
            snapshot: feeder::Snapshot::default(),
            #[cfg(test)]
            next: None,
        }
    }

    /// Hands the changed pad layout to the input thread; what the
    /// Controls page says (a save that failed also goes to `launcher.log`).
    fn bindings_changed(&self, done: Result<String, String>) -> Note {
        if let Some(i) = &self.input {
            i.set_pad_map(self.bindings.map().clone());
        }
        match done {
            Ok(said) => Note::ok(said),
            Err(e) => {
                self.log.error(&e);
                Note::error(e)
            }
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
        Setup::DirectX => (
            "Installing Microsoft DirectX audio…",
            "Initial D 6, 7 and 8 play their sound through DirectX's XAudio2 2.3, \
             installed once from Microsoft's DirectX setup. Windows asks for your \
             permission first.",
        ),
    }
}

impl HubBackend for Launcher {
    type Library = Scan<Game>;
    type Form = Form;

    fn games(&self) -> &[HubGame] {
        &self.games
    }

    fn blurb(&self, i: usize) -> Option<&'static str> {
        Some(GAMES[i].blurb)
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

    fn form(&mut self, game: usize, screen: Option<(u32, u32)>) -> Form {
        let g = &GAMES[game];
        let file = self.paths.settings(g.slug);
        let settings = match Settings::load(&file, g.native, screen) {
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
        Form::new(settings, g.native, screen)
    }

    fn session(
        &mut self,
        game: usize,
        library: &Scan<Game>,
        config: &HubConfig,
        screen: Option<(u32, u32)>,
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
        let (settings, _) = Settings::load(&file, g.native, screen)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let feed = match (g.route, &self.input) {
            (Route::Wal { .. }, Some(input)) => Feed::Devices {
                tx: input.sender(),
                deadzone: settings.deadzone,
            },
            _ => Feed::None,
        };
        let (paths, config) = (self.paths.clone(), config.clone());
        Ok(Box::new(move |r: &Reporter| {
            let report = |setup| {
                let (title, body) = stage(setup);
                r.setting_up(title, body);
            };
            let ran = session::play(
                &paths,
                g,
                &entry,
                &settings,
                &config,
                Display::Screen,
                &feed,
                &r.stop_flag(),
                report,
                || r.playing(),
            )
            .map_err(|e| format!("{e:#}"))?;
            ran.map_or(Ok(()), |(prepared, out)| session::verdict(&prepared, &out))
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
        controls_view(&self.snapshot(), &self.bindings)
    }

    fn pad(&mut self) -> Vec<Input> {
        let bits = self.snapshot().nav;
        Input::from_pad(&mut self.pad, bits)
    }

    fn pad_now(&self) -> Option<(u16, Option<PadInput>)> {
        let snap = self.snapshot();
        Some((snap.nav, snap.held))
    }

    fn rebindable(&self) -> Vec<usize> {
        self.bindings.rebindable()
    }

    fn rebind(&mut self, row: usize, input: PadInput) -> Note {
        let file = self.paths.controls_file();
        let done = self.bindings.rebind_saved(row, input, &file);
        self.bindings_changed(done)
    }

    fn reset_bindings(&mut self) -> Note {
        let file = self.paths.controls_file();
        let done = self.bindings.reset_saved(&file);
        self.bindings_changed(done)
    }
}

/// The Controls page and the top bar from the input thread's snapshot and
/// the player's pad layout.
fn controls_view(snap: &feeder::Snapshot, bindings: &Bindings<Button>) -> ControlsView {
    let layout = bindings.layout();
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
        lines: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
