//! The input thread (`rea_launcher_native::feeder`) with the cabinet's pad
//! layout, and windowsarcadeloader's line format as a [`Wire`]: while a
//! Windows game runs, its stick lines go to the loader's input pipe and
//! the force feedback it sends back to the wheel. The native loaders read
//! SDL themselves.

use initiald_core::input::{Button, StickLayout, default_pad_map};
use rea_launcher::hub::HubPaths;
use rea_launcher::hub::bindings::Bindings;
use rea_launcher_native::feeder::{Frame, InputService, Wire};

/// The player's pad layout: the cabinet's, with the changes saved in the
/// data folder's `controls.cfg`. A file that can't be read leaves the
/// cabinet's layout, and `warn` hears why.
pub fn bindings(paths: &HubPaths, warn: impl FnOnce(&str)) -> Bindings<Button> {
    let file = paths.controls_file();
    Bindings::load(&file, default_pad_map()).unwrap_or_else(|e| {
        warn(&format!(
            "{}: {e}; the default pad layout is used",
            file.display()
        ));
        Bindings::new(default_pad_map())
    })
}

/// Starts the input thread with the player's pad layout `bindings`,
/// preferring a saved `wheel.ini` in the data folder when present.
pub fn start(paths: &HubPaths, bindings: &Bindings<Button>) -> InputService {
    InputService::start_with_wheel_file(
        bindings.map().clone(),
        Some((Button::GearUp, Button::GearDown)),
        Some(paths.wheel_file()),
    )
}

/// windowsarcadeloader's stick line for one system profile family.
pub struct Wal(pub StickLayout);

impl Wire for Wal {
    fn encode(&mut self, f: &Frame) -> String {
        self.0.pipe_line_with_dpad(&f.controls, f.dpad) + "\n"
    }

    fn quit(&self) -> Option<&'static str> {
        Some("quit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::wheel::{Control, Controls};

    #[test]
    fn a_frame_is_one_line_of_the_profiles_stick() {
        let frame = Frame {
            controls: Controls {
                buttons: Button::View.bit(),
                accel: 1.0,
                ..Controls::default()
            },
            dpad: rea_launcher::pad::DPAD_UP | rea_launcher::pad::DPAD_RIGHT,
            ..Frame::default()
        };
        assert_eq!(
            Wal(StickLayout::RingEdge).encode(&frame),
            "p1 49 0 0 0 0 32767 0\n"
        );
        assert_eq!(
            Wal(StickLayout::Alls).encode(&frame),
            "p1 19 0 0 0 0 32767 0\n"
        );
        assert_eq!(Wal(StickLayout::Alls).quit(), Some("quit"));
    }
}
