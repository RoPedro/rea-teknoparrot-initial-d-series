//! A game's settings page: what each row of [`settings::OPTIONS`] shows,
//! why one is off, and how the arrows change the [`Settings`] behind it.
//! The Look row sets the picture rows at once; changing one of those by
//! hand makes the look "Custom". The `crt` key has no row (command line
//! only until it is checked on a real screen); a save writes only the rows
//! the player changed.

use std::io;
use std::path::Path;

use initiald_core::settings::{self, LOOK_KEYS, Look, Settings};
use rea_launcher::gamescope::{Size, Upscaler};
use rea_launcher::hub::{pick, size_name};
use rea_launcher_ui::hub::HubForm;

/// Output sizes offered in the window; any other saved size is kept.
const OUTPUTS: [Size; 3] = [
    Size::new(1920, 1080),
    Size::new(2560, 1440),
    Size::new(3840, 2160),
];

/// Render sizes (`None` = the cabinet's own size).
const RENDERS: [Option<Size>; 4] = [
    None,
    Some(Size::new(1920, 1080)),
    Some(Size::new(2560, 1440)),
    Some(Size::new(3840, 2160)),
];

const SHARPNESS_OFF: &str = "Only FSR 1 and NVIDIA Image Scaling use sharpness.";

#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub settings: Settings,
    /// The cabinet's own screen size, for "native" and the looks.
    native: Size,
    /// The player's screen, for the looks.
    screen: Option<(u32, u32)>,
    /// The keys the player changed: the only ones a save writes.
    changed: Vec<&'static str>,
}

fn area(s: &Size) -> u64 {
    u64::from(s.w) * u64::from(s.h)
}

/// The curated output sizes plus the current one, smallest first.
fn output_choices(current: Size) -> Vec<Size> {
    let mut v = OUTPUTS.to_vec();
    if !v.contains(&current) {
        v.push(current);
        v.sort_by_key(area);
    }
    v
}

/// The curated render sizes plus the current one.
fn render_choices(current: Option<Size>) -> Vec<Option<Size>> {
    let mut v = RENDERS.to_vec();
    if !v.contains(&current) {
        v.push(current);
        v[1..].sort_by_key(|s| s.as_ref().map(area));
    }
    v
}

impl Form {
    pub fn new(settings: Settings, native: Size, screen: Option<(u32, u32)>) -> Self {
        Self {
            settings,
            native,
            screen,
            changed: Vec::new(),
        }
    }

    fn changed(&mut self, keys: &[&'static str]) {
        for key in keys {
            if !self.changed.contains(key) {
                self.changed.push(key);
            }
        }
    }
}

impl HubForm for Form {
    fn value(&self, key: &str) -> String {
        let s = &self.settings;
        match key {
            "look" => s.look.map_or("Custom", Look::label).to_string(),
            "render" => match s.render {
                None => format!("Native, {}", size_name(self.native)),
                Some(z) => size_name(z),
            },
            "output" => size_name(s.output),
            "upscaler" => s.upscaler.label().to_string(),
            "sharpness" => s.sharpness.to_string(),
            "fullscreen" => if s.fullscreen { "On" } else { "Off" }.to_string(),
            "deadzone" => format!("{:.0}%", s.deadzone * 100.0),
            _ => String::new(),
        }
    }

    fn disabled(&self, key: &str) -> Option<String> {
        (key == "sharpness" && !self.settings.upscaler.sharpens()).then(|| SHARPNESS_OFF.into())
    }

    fn adjust(&mut self, key: &str, d: i32, wrap: bool) -> bool {
        let Some(spec) = settings::OPTIONS.iter().find(|o| o.key == key) else {
            return false;
        };
        if self.disabled(key).is_some() || d == 0 {
            return false;
        }
        let before = self.settings.clone();
        let s = &mut self.settings;
        match key {
            "look" => {
                let next = match s.look {
                    // from a custom picture, the one for this screen first
                    None => Look::recommended(self.screen),
                    Some(cur) => pick(&Look::ALL, cur, d, wrap),
                };
                next.apply(s, self.native, self.screen);
            }
            "render" => s.render = pick(&render_choices(s.render), s.render, d, wrap),
            "output" => s.output = pick(&output_choices(s.output), s.output, d, wrap),
            "upscaler" => s.upscaler = pick(&Upscaler::ALL, s.upscaler, d, wrap),
            "fullscreen" => s.fullscreen = !s.fullscreen,
            "sharpness" => s.sharpness = (i32::from(s.sharpness) + d).clamp(0, 20) as u8,
            "deadzone" => {
                let pct = (s.deadzone * 100.0).round() as i32 + d;
                s.deadzone = pct.clamp(0, 30) as f32 / 100.0;
            }
            _ => return false,
        }
        if self.settings == before {
            return false;
        }
        if spec.key == "look" {
            self.changed(&LOOK_KEYS);
        } else {
            self.changed(&[spec.key]);
            if LOOK_KEYS.contains(&spec.key) || matches!(spec.key, "sharpness" | "fullscreen") {
                self.settings.look = None;
                self.changed(&["look"]);
            }
        }
        true
    }

    fn save(&self, file: &Path) -> io::Result<()> {
        self.settings.write(file, &self.changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> Form {
        Form::new(Settings::default(), Size::new(1360, 768), None)
    }

    #[test]
    fn output_steps_through_the_curated_sizes_and_keeps_a_custom_one() {
        let mut f = form();
        assert_eq!(f.settings.output, Size::new(3840, 2160));
        assert!(!f.adjust("output", 1, false), "clamped at the end");
        assert!(f.adjust("output", -1, false));
        assert_eq!(f.settings.output, Size::new(2560, 1440));
        assert!(f.adjust("output", 1, true));
        assert!(f.adjust("output", 1, true), "Enter wraps around");
        assert_eq!(f.settings.output, Size::new(1920, 1080));

        let custom = Size::new(3440, 1440);
        assert_eq!(
            output_choices(custom),
            vec![
                Size::new(1920, 1080),
                Size::new(2560, 1440),
                custom,
                Size::new(3840, 2160)
            ]
        );
        let mut f = form();
        f.settings.output = custom;
        assert_eq!(f.value("output"), "3440 × 1440");
        f.adjust("output", 1, false);
        assert_eq!(f.settings.output, Size::new(3840, 2160));
    }

    #[test]
    fn render_size_starts_native_and_steps_up() {
        let mut f = form();
        assert_eq!(f.value("render"), "Native, 1360 × 768");
        for _ in 0..4 {
            f.adjust("render", 1, false);
        }
        assert_eq!(f.settings.render, Some(Size::new(3840, 2160)));
        assert_eq!(f.value("render"), "3840 × 2160 (4K)");
        f.adjust("render", -9, false);
        assert_eq!(f.settings.render, None);
        assert!(f.adjust("render", -1, true), "Enter wraps back");
        assert_eq!(f.settings.render, Some(Size::new(3840, 2160)));
    }

    #[test]
    fn sharpness_is_off_unless_fsr_or_nis() {
        let mut f = form();
        assert!(f.disabled("sharpness").is_none());
        assert!(f.adjust("sharpness", 1, false));
        assert_eq!(f.settings.sharpness, 6);
        // FSR -> NIS -> Smooth
        f.adjust("upscaler", 1, false);
        assert_eq!(f.settings.upscaler, Upscaler::Nis);
        f.adjust("upscaler", 1, false);
        assert_eq!(f.value("upscaler"), "Smooth");
        assert_eq!(f.disabled("sharpness").as_deref(), Some(SHARPNESS_OFF));
        assert!(!f.adjust("sharpness", 1, false));
        assert_eq!(f.settings.sharpness, 6);
    }

    #[test]
    fn sharpness_and_dead_zone_clamp() {
        let mut f = form();
        for _ in 0..40 {
            f.adjust("sharpness", 1, false);
            f.adjust("deadzone", 1, false);
        }
        assert_eq!(f.settings.sharpness, 20);
        assert!((f.settings.deadzone - 0.30).abs() < 1e-6);
        assert_eq!(f.value("deadzone"), "30%");
        for _ in 0..40 {
            f.adjust("deadzone", -1, false);
        }
        assert_eq!(f.settings.deadzone, 0.0);
    }

    #[test]
    fn every_row_has_a_value_and_crt_has_no_row() {
        let f = form();
        for o in settings::OPTIONS.iter().filter(|o| !o.launcher_only) {
            assert!(!f.value(o.key).is_empty(), "{}", o.key);
        }
        let crt = settings::OPTIONS.iter().find(|o| o.key == "crt").unwrap();
        assert!(crt.launcher_only);
        assert!(!form().adjust("crt", 1, false));
    }

    #[test]
    fn a_look_sets_the_picture_and_a_hand_change_makes_it_custom() {
        let mut f = Form::new(Settings::default(), Size::new(1360, 768), Some((1280, 800)));
        assert_eq!(f.value("look"), "Custom");
        assert!(f.adjust("look", 1, false), "from custom: the screen's look");
        assert_eq!(f.value("look"), "Steam Deck");
        assert_eq!(f.settings.output, Size::new(1280, 800));
        assert!(f.adjust("look", -1, false));
        assert_eq!(f.value("look"), "Recommended");
        assert!(
            f.adjust("look", 1, true) && f.adjust("look", 1, true),
            "Enter wraps"
        );
        assert_eq!(f.settings.look, Some(Look::Original));
        assert_eq!(f.settings.render, None);
        f.adjust("upscaler", 1, false);
        assert_eq!(f.value("look"), "Custom");
        f.adjust("look", 1, false);
        f.adjust("deadzone", 1, false);
        assert_eq!(
            f.value("look"),
            "Steam Deck",
            "the dead zone is not a picture value"
        );
    }

    #[test]
    fn saving_writes_only_the_changed_rows_and_keeps_the_rest() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("settings/id6.cfg");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = "# my notes\nfuture = 1\ncrt = lottes\nfullscreen = maybe\n";
        std::fs::write(&path, text).unwrap();
        let mut f = Form::new(
            Settings::load(&path, Size::new(1360, 768), None).unwrap().0,
            Size::new(1360, 768),
            None,
        );
        f.adjust("output", -2, false);
        f.save(&path).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, format!("{text}output = 1920x1080\n"));
        // an unreadable file is left alone
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(f.save(&path).is_err());
    }
}
