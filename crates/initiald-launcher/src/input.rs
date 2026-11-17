//! The input thread (`rea_launcher_native::feeder`) with the cabinet's pad
//! layout, and windowsarcadeloader's line format as a [`Wire`]: while a
//! Windows game runs, its stick lines go to the loader's input pipe and
//! the force feedback it sends back to the wheel. The native loaders read
//! SDL themselves.

use initiald_core::input::{Button, StickLayout, default_pad_map};
use rea_launcher_native::feeder::{Frame, InputService, Wire};

/// Starts the input thread.
pub fn start() -> InputService {
    InputService::start(default_pad_map(), Some((Button::GearUp, Button::GearDown)))
}

/// windowsarcadeloader's stick line for one system profile family.
pub struct Wal(pub StickLayout);

impl Wire for Wal {
    fn encode(&mut self, f: &Frame) -> String {
        self.0.pipe_line(&f.controls) + "\n"
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
            ..Frame::default()
        };
        assert_eq!(
            Wal(StickLayout::RingEdge).encode(&frame),
            "p1 40 0 0 0 0 32767 0\n"
        );
        assert_eq!(Wal(StickLayout::Alls).quit(), Some("quit"));
    }
}
