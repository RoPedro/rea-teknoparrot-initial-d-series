//! The cabinet's buttons, the default pad layout, and the step from the
//! SDK's device-independent controls to what each loader reads.
//!
//! windowsarcadeloader's payloads read a virtual arcade stick per player
//! (its `wal_protocol`): buttons as bits, six axes. Which bit is which
//! cabinet button depends on the system profile; [`StickLayout`] holds the
//! two the Initial D profiles use (read from their `native_map`/`gamepad`
//! sections at the pinned commit).

use rea_launcher::wheel::{Action, Control, Controls as Read, PadInput, PadMap};

/// Every button an Initial D cabinet has (besides wheel and pedals), plus
/// the sequential shift a pad or paddles use instead of the H lever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Button {
    GearUp,
    GearDown,
    View,
    Start,
    Card,
    Coin,
    Test,
    Service,
}

impl Control for Button {
    const ALL: &'static [Self] = &[
        Button::GearUp,
        Button::GearDown,
        Button::View,
        Button::Start,
        Button::Card,
        Button::Coin,
        Button::Test,
        Button::Service,
    ];

    fn key(self) -> &'static str {
        match self {
            Button::GearUp => "gear_up",
            Button::GearDown => "gear_down",
            Button::View => "view",
            Button::Start => "start",
            Button::Card => "card",
            Button::Coin => "coin",
            Button::Test => "test",
            Button::Service => "service",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Button::GearUp => "Shift up",
            Button::GearDown => "Shift down",
            Button::View => "View change",
            Button::Start => "Start",
            Button::Card => "Insert / remove card",
            Button::Coin => "Insert coin",
            Button::Test => "Operator test menu",
            Button::Service => "Service",
        }
    }

    fn bit(self) -> u32 {
        1 << Self::ALL.iter().position(|&b| b == self).expect("in ALL")
    }
}

/// The pad layout a player gets before changing anything: triggers for the
/// pedals, the left stick steers, shoulders shift, Y changes the view.
pub fn default_pad_map() -> PadMap<Button> {
    PadMap::with(&[
        (Action::Accelerate, &[PadInput::RightTrigger]),
        (Action::Brake, &[PadInput::LeftTrigger]),
        (Action::Press(Button::GearUp), &[PadInput::RightShoulder]),
        (Action::Press(Button::GearDown), &[PadInput::LeftShoulder]),
        (Action::Press(Button::View), &[PadInput::Y]),
        (
            Action::Press(Button::Start),
            &[PadInput::Start, PadInput::A],
        ),
        (Action::Press(Button::Card), &[PadInput::X]),
        (Action::Press(Button::Coin), &[PadInput::Back]),
    ])
}

/// windowsarcadeloader's virtual stick bits (`wal_protocol::button`).
pub mod wal {
    pub const B1: u32 = 1 << 4;
    pub const B2: u32 = 1 << 5;
    pub const B3: u32 = 1 << 6;
    pub const B6: u32 = 1 << 9;
    pub const B7: u32 = 1 << 10;
    pub const START: u32 = 1 << 12;
    pub const COIN: u32 = 1 << 13;
    pub const SERVICE: u32 = 1 << 14;
    pub const TEST: u32 = 1 << 15;
    pub const CARD: u32 = 1 << 16;
}

/// Which virtual stick buttons a family of system profiles reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickLayout {
    /// RingEdge (ID6–ID8): b1/b2 shift up/down, b3 view change.
    RingEdge,
    /// Nu and ALLS (Zero, THE ARCADE): b7/b6 shift up/down, b1 view change.
    Alls,
}

impl StickLayout {
    fn bit(self, b: Button) -> u32 {
        use StickLayout::*;
        match (self, b) {
            (RingEdge, Button::GearUp) => wal::B1,
            (RingEdge, Button::GearDown) => wal::B2,
            (RingEdge, Button::View) => wal::B3,
            (Alls, Button::GearUp) => wal::B7,
            (Alls, Button::GearDown) => wal::B6,
            (Alls, Button::View) => wal::B1,
            (_, Button::Start) => wal::START,
            (_, Button::Card) => wal::CARD,
            (_, Button::Coin) => wal::COIN,
            (_, Button::Test) => wal::TEST,
            (_, Button::Service) => wal::SERVICE,
        }
    }

    /// Player 1's line for `arcade-launcher --input-pipe`
    /// (`p1 <buttons hex> <lx> <ly> <rx> <ry> <accel> <brake>`).
    pub fn pipe_line(self, r: &Read) -> String {
        let buttons = Button::ALL
            .iter()
            .filter(|b| r.buttons & b.bit() != 0)
            .fold(0u32, |acc, &b| acc | self.bit(b));
        let axis = |v: f32| (v.clamp(-1.0, 1.0) * 32767.0).round() as i32;
        let pedal = |v: f32| (v.clamp(0.0, 1.0) * 32767.0).round() as i32;
        format!(
            "p1 {buttons:x} {} 0 0 0 {} {}",
            axis(r.steer),
            pedal(r.accel),
            pedal(r.brake)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::wheel::{PadSettings, PadState, read_pad};

    #[test]
    fn bits_are_distinct() {
        let mut seen = 0u32;
        for b in Button::ALL {
            assert_eq!(seen & b.bit(), 0);
            seen |= b.bit();
        }
    }

    #[test]
    fn each_layout_maps_every_button_to_its_own_bit() {
        for layout in [StickLayout::RingEdge, StickLayout::Alls] {
            let mut seen = 0u32;
            for &b in Button::ALL {
                let bit = layout.bit(b);
                assert_eq!(seen & bit, 0, "{layout:?} {b:?}");
                seen |= bit;
            }
        }
    }

    #[test]
    fn view_and_shifts_follow_the_profile_family() {
        let r = Read {
            buttons: Button::View.bit() | Button::GearUp.bit(),
            ..Read::default()
        };
        assert_eq!(StickLayout::RingEdge.pipe_line(&r), "p1 50 0 0 0 0 0 0");
        assert_eq!(StickLayout::Alls.pipe_line(&r), "p1 410 0 0 0 0 0 0");
    }

    #[test]
    fn a_pad_steers_and_presses_the_pedals() {
        let settings = PadSettings::new(default_pad_map());
        let pad = PadState {
            left_x: -32768,
            right_trigger: 32767,
            start: true,
            ..PadState::default()
        };
        let read = read_pad(&settings, &pad, None);
        let line = StickLayout::RingEdge.pipe_line(&read);
        assert_eq!(line, "p1 1000 -32767 0 0 0 32767 0");
    }
}
