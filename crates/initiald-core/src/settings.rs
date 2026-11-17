//! Per-game settings, stored as `key = value` lines in
//! `<data>/settings/<slug>.cfg`.
//!
//! [`OPTIONS`] names every setting once (`rea_launcher::cfg` option specs):
//! its key, its flag, the values it takes, and the label and description
//! the window and `--help` show. Each key keeps the format earlier versions
//! wrote (`fullscreen = true`, `deadzone = 0.05`). A save writes only the
//! keys that were set, so the player's comments, unknown keys and any line
//! the launcher couldn't read stay as they were; an unreadable file is
//! never written over.

use std::io;
use std::path::Path;

use rea_launcher::cfg::{self, KeyStyle, Kind, OptionSpec, Rejected, Resolution, Value};
use rea_launcher::gamescope::{
    self, CRTS, Crt, SIZE_MAX, SIZE_MIN, Size, UPSCALERS, Upscaler, fit,
};
use rea_launcher::{display, fsx};

/// Flags are the keys: `--render` is `render`.
pub const STYLE: KeyStyle = KeyStyle::Underscore;

/// A look: a named set of picture values (the render size, the size on
/// the screen, the upscaler).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    /// The game as the cabinet showed it: its own size, smooth scaling.
    Original,
    /// Drawn at the player's screen size in the cabinet's shape, FSR to fill
    /// the rest.
    Recommended,
    /// The Steam Deck's 1280×800 screen.
    SteamDeck,
}

pub const LOOKS: &[(&str, &str)] = &[
    ("original", "Original"),
    ("recommended", "Recommended"),
    ("steam_deck", "Steam Deck"),
];

/// The keys a look sets.
pub const LOOK_KEYS: [&str; 4] = ["look", "render", "output", "upscaler"];

impl Look {
    pub const ALL: [Look; 3] = [Look::Original, Look::Recommended, Look::SteamDeck];

    pub fn key(self) -> &'static str {
        LOOKS[self as usize].0
    }

    pub fn label(self) -> &'static str {
        LOOKS[self as usize].1
    }

    pub fn from_key(key: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|l| l.key() == key)
    }

    /// The look for `screen` (the monitor in pixels, when known): Steam
    /// Deck on a Deck's screen.
    pub fn recommended(screen: Option<(u32, u32)>) -> Look {
        if display::is_steam_deck(screen) {
            Look::SteamDeck
        } else {
            Look::Recommended
        }
    }

    /// Sets this look's values in `s` for a cabinet of size `native` on
    /// `screen` (1920×1080 when unknown).
    pub fn apply(self, s: &mut Settings, native: Size, screen: Option<(u32, u32)>) {
        let screen = screen
            .and_then(display::landscape)
            .map_or(Size::new(1920, 1080), |(w, h)| Size::new(w, h));
        let (render, output, upscaler) = match self {
            Look::Original => (None, screen, Upscaler::Linear),
            Look::Recommended => (Some(fit(native, screen)), screen, Upscaler::Fsr),
            Look::SteamDeck => {
                let (w, h) = display::STEAM_DECK_SCREEN;
                let deck = Size::new(w, h);
                (Some(fit(native, deck)), deck, Upscaler::Linear)
            }
        };
        s.render = render;
        s.output = output;
        s.upscaler = upscaler;
        s.look = Some(self);
    }
}

const NATIVE: &cfg::Aliases = &[("native", Resolution::Original)];

/// Every per-game option: settings key, flag and window row.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "Sets the picture at once: the cabinet's own, sharper for your screen, or for the \
         Steam Deck. Options set after it win.",
    ),
    OptionSpec::new(
        "render",
        Kind::Size {
            min: SIZE_MIN,
            max: SIZE_MAX,
            aliases: NATIVE,
        },
        "Game render size",
        "The size the game draws at. Bigger is sharper and needs a stronger GPU.",
    )
    .with_placeholder("<WxH|native>")
    .with_example("3840x2160"),
    OptionSpec::new(
        "output",
        Kind::Size {
            min: SIZE_MIN,
            max: SIZE_MAX,
            aliases: cfg::SIZES_ONLY,
        },
        "Output size",
        "The size of the picture on your screen. Pick your screen's size.",
    )
    .with_example("3840x2160")
    .helper_owned(),
    OptionSpec::new(
        "upscaler",
        Kind::Choice(UPSCALERS),
        "Upscaler",
        "How the game's picture is stretched to the output size.",
    )
    .helper_owned(),
    OptionSpec::new(
        "sharpness",
        Kind::Range { min: 0, max: 20 },
        "Sharpness",
        "How crisp the upscaler makes edges. 0 is the sharpest.",
    )
    .helper_owned(),
    // command line only until it is checked on a real screen
    OptionSpec::new(
        "crt",
        Kind::Choice(CRTS),
        "CRT look",
        "CRT scanlines and mask drawn over the game.",
    )
    .helper_owned()
    .launcher_only(),
    OptionSpec::new(
        "fullscreen",
        Kind::Toggle,
        "Fullscreen",
        "Fill the whole screen, or play in a window.",
    )
    .helper_owned(),
    OptionSpec::new(
        "deadzone",
        Kind::Decimal {
            min: 0,
            max: 30,
            places: 2,
        },
        "Dead zone",
        "How far a controller stick moves before the car starts to steer.",
    )
    .with_example("0.05"),
];

/// Comment lines at the top of a new settings file.
pub const HEADING: &str = "Settings for one game of the Initial D launcher.\n\
    Every key is also a command-line option: render = 2560x1440 is \
    --play <game> --render 2560x1440.";

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The look last chosen (its values are in the fields below); `None`
    /// once a picture value was changed by hand.
    pub look: Option<Look>,
    /// The size the game itself draws at; `None`: the cabinet's own.
    pub render: Option<Size>,
    /// What gamescope shows on the screen.
    pub output: Size,
    pub upscaler: Upscaler,
    /// 0–20, gamescope's FSR/NIS sharpness (0 = sharpest).
    pub sharpness: u8,
    pub crt: Crt,
    pub fullscreen: bool,
    /// Pad stick dead zone around the centre, 0.0–0.3.
    pub deadzone: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            look: None,
            render: None,
            output: Size::new(3840, 2160),
            upscaler: Upscaler::Fsr,
            sharpness: 5,
            crt: Crt::Off,
            fullscreen: true,
            deadzone: 0.05,
        }
    }
}

impl Settings {
    /// Sets one key from a value its row checked (the file, the command
    /// line and the window all come through here).
    pub fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        match (key, value) {
            ("look", Value::Choice(v)) => self.look = Look::from_key(v),
            ("render", Value::Size(Resolution::Size(w, h))) => self.render = Some(Size::new(w, h)),
            ("render", Value::Size(_)) => self.render = None,
            ("output", Value::Size(Resolution::Size(w, h))) => self.output = Size::new(w, h),
            ("upscaler", Value::Choice(v)) => {
                self.upscaler = Upscaler::parse(v).ok_or(Rejected::Unknown)?
            }
            ("sharpness", Value::Number(n)) => self.sharpness = n as u8,
            ("crt", Value::Choice(v)) => self.crt = Crt::parse(v).ok_or(Rejected::Unknown)?,
            ("fullscreen", Value::Toggle(b)) => self.fullscreen = b,
            ("deadzone", Value::Decimal(d)) => self.deadzone = d.as_f32(),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// Sets `key` from its text, checked against its row.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), Rejected> {
        cfg::spec::dispatch(OPTIONS, key, value, &mut |k, v| self.apply(k, v))
    }

    /// Reads a settings file's text; a bad value keeps the default and is
    /// reported.
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut s = Settings::default();
        let problems = cfg::spec::parse_file(text, "settings", OPTIONS, |k, v| s.apply(k, v));
        (s, problems)
    }

    /// The file value of `key`, in the format earlier versions wrote
    /// (`look`: empty when there is none).
    pub fn value(&self, key: &str) -> String {
        match key {
            "look" => self.look.map_or("", Look::key).into(),
            "render" => self.render.map_or("native".into(), |z| z.to_string()),
            "output" => self.output.to_string(),
            "upscaler" => self.upscaler.key().into(),
            "sharpness" => self.sharpness.to_string(),
            "crt" => self.crt.key().into(),
            "fullscreen" => self.fullscreen.to_string(),
            "deadzone" => format!("{:.2}", self.deadzone),
            _ => unreachable!("unknown settings key {key}"),
        }
    }

    /// `existing` with `keys` set to these settings' values, every other
    /// line as it was (no look: its line is cleared); a new file starts
    /// from the commented template and gets every key.
    pub fn save(&self, existing: &str, keys: &[&str]) -> String {
        let fresh = existing.trim().is_empty();
        let mut out = if fresh {
            template()
        } else {
            existing.to_string()
        };
        for spec in OPTIONS.iter().filter(|s| fresh || keys.contains(&s.key)) {
            out = match self.value(spec.key) {
                v if v.is_empty() => cfg::clear_key(&out, spec.key),
                v => cfg::spec::set_key(&out, spec, &v),
            };
        }
        out
    }

    /// The settings in `file` of a game whose cabinet is `native` in size,
    /// and what couldn't be read in it; an unreadable file is an error. The
    /// first time (no file yet) the recommended look for `screen` is
    /// decided and saved, so a Play or `--play` before the settings page
    /// ever opened uses it too.
    pub fn load(
        file: &Path,
        native: Size,
        screen: Option<(u32, u32)>,
    ) -> io::Result<(Settings, Vec<String>)> {
        if let Some(text) = cfg::read_file(file)? {
            return Ok(Settings::parse(&text));
        }
        let mut s = Settings::default();
        Look::recommended(screen).apply(&mut s, native, screen);
        let problems = match s.write(file, &[]) {
            Ok(()) => Vec::new(),
            Err(e) => vec![format!(
                "couldn't save the recommended look to {}: {e}",
                file.display()
            )],
        };
        Ok((s, problems))
    }

    /// Writes `keys` into `file` ([`Settings::save`]) as a whole-file swap;
    /// a file that can't be read is left alone and the error returned.
    pub fn write(&self, file: &Path, keys: &[&str]) -> io::Result<()> {
        let existing = cfg::read_file(file)?.unwrap_or_default();
        fsx::write_atomic(file, self.save(&existing, keys))
    }

    /// The picture gamescope shows of a game drawn at `game`.
    pub fn picture(&self, game: Size) -> gamescope::Picture {
        gamescope::Picture {
            game,
            output: self.output,
            upscaler: self.upscaler,
            sharpness: self.sharpness,
            fullscreen: self.fullscreen,
            crt: self.crt,
        }
    }
}

/// The commented file a new settings file starts from.
pub fn template() -> String {
    cfg::spec::template(HEADING, OPTIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_comments_and_unknown_keys() {
        let text = "# my notes\nupscaler = nis\nfuture_key = 7\noutput=2560x1440\ncrt = lottes\n";
        let (s, problems) = Settings::parse(text);
        assert_eq!(problems.len(), 1, "only the unknown key: {problems:?}");
        assert_eq!(s.upscaler, Upscaler::Nis);
        assert_eq!(s.output, Size::new(2560, 1440));
        assert_eq!(s.crt, Crt::Lottes);
        let saved = s.save(text, &["render", "output"]);
        assert_eq!(
            saved,
            "# my notes\nupscaler = nis\nfuture_key = 7\noutput = 2560x1440\ncrt = lottes\n\
             render = native\n"
        );
        assert_eq!(Settings::parse(&saved).0, s);
    }

    #[test]
    fn bad_values_fall_back_and_are_reported() {
        let (s, p) = Settings::parse(
            "output = 99x99\nsharpness = 50\nfullscreen = maybe\ncrt = rainbow\ndeadzone = 0.5\n",
        );
        assert_eq!(s, Settings::default());
        assert_eq!(p.len(), 5, "{p:?}");
    }

    #[test]
    fn a_bad_value_stays_as_the_player_wrote_it_unless_set() {
        let text = "fullscreen = off # trailing\ndeadzone = 0.123\n";
        let (mut s, problems) = Settings::parse(text);
        assert_eq!(problems.len(), 1, "{problems:?}");
        s.sharpness = 3;
        assert_eq!(
            s.save(text, &["sharpness"]),
            "fullscreen = off # trailing\ndeadzone = 0.123\nsharpness = 3\n",
            "neither the unread line nor 0.123's precision is rewritten"
        );
        s.fullscreen = false;
        assert!(
            s.save(text, &["fullscreen"])
                .starts_with("fullscreen = false\n")
        );
    }

    #[test]
    fn each_key_keeps_its_stored_format() {
        let mut s = Settings::default();
        for (key, value) in [
            ("fullscreen", "off"),
            ("deadzone", "0.1"),
            ("render", "NATIVE"),
        ] {
            s.set(key, value).unwrap();
        }
        assert_eq!(s.value("fullscreen"), "false");
        assert_eq!(s.value("deadzone"), "0.10");
        assert_eq!(s.value("render"), "native");
        assert!(s.set("deadzone", "5").is_err());
    }

    #[test]
    fn a_new_file_is_the_template_with_every_key() {
        let mut s = Settings::default();
        Look::Original.apply(&mut s, Size::new(1360, 768), None);
        let text = s.save("", &[]);
        assert!(text.starts_with("# Settings for one game"), "{text}");
        for spec in OPTIONS {
            assert!(
                text.contains(&format!("\n{} = {}\n", spec.key, s.value(spec.key))),
                "{}",
                spec.key
            );
        }
        assert_eq!(Settings::parse(&text), (s, vec![]));
    }

    #[test]
    fn an_unreadable_file_is_not_written_over() {
        let t = tempfile::tempdir().unwrap();
        let file = t.path().join("id6.cfg");
        std::fs::create_dir(&file).unwrap();
        assert!(Settings::load(&file, Size::new(1360, 768), None).is_err());
        assert!(Settings::default().write(&file, &["render"]).is_err());
        assert!(file.is_dir());
    }

    const RINGEDGE: Size = Size::new(1360, 768);

    #[test]
    fn looks_set_the_picture_for_the_screen_in_the_cabinets_shape() {
        let mut s = Settings::default();
        Look::Recommended.apply(&mut s, Size::new(640, 480), Some((3840, 2160)));
        assert_eq!(
            (s.render, s.output, s.upscaler),
            (
                Some(Size::new(2880, 2160)),
                Size::new(3840, 2160),
                Upscaler::Fsr
            ),
            "4:3 inside 16:9"
        );
        Look::SteamDeck.apply(&mut s, RINGEDGE, Some((3840, 2160)));
        assert_eq!(
            (s.render, s.output, s.upscaler),
            (
                Some(Size::new(1280, 722)),
                Size::new(1280, 800),
                Upscaler::Linear
            )
        );
        Look::Original.apply(&mut s, RINGEDGE, Some((2560, 1440)));
        assert_eq!(
            (s.render, s.output, s.upscaler),
            (None, Size::new(2560, 1440), Upscaler::Linear),
            "the cabinet's own size, smooth"
        );
        assert_eq!(s.look, Some(Look::Original));
        assert_eq!(Look::recommended(Some((800, 1280))), Look::SteamDeck);
        assert_eq!(Look::recommended(None), Look::Recommended);
    }

    #[test]
    fn a_look_given_up_is_cleared_from_the_file() {
        let mut s = Settings::default();
        Look::Original.apply(&mut s, RINGEDGE, None);
        let text = s.save("", &[]);
        assert!(text.contains("\nlook = original\n"), "{text}");
        s.look = None;
        s.upscaler = Upscaler::Nis;
        let text = s.save(&text, &["look", "upscaler"]);
        assert!(!text.contains("\nlook ="), "{text}");
        assert_eq!(Settings::parse(&text).0, s);
    }

    #[test]
    fn the_first_load_decides_the_recommended_look_and_saves_it() {
        let t = tempfile::tempdir().unwrap();
        let file = t.path().join("settings/id6.cfg");
        let (s, problems) = Settings::load(&file, RINGEDGE, Some((1280, 800))).unwrap();
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            (s.look, s.output),
            (Some(Look::SteamDeck), Size::new(1280, 800))
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(Settings::parse(&text).0, s, "saved");
        // a second load reads the file, whatever the screen
        assert_eq!(
            Settings::load(&file, RINGEDGE, Some((3840, 2160)))
                .unwrap()
                .0,
            s
        );
    }
}
