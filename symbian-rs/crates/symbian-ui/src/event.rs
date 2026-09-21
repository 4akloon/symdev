//! Keys, as they arrive from the window server through `CCoeControl::OfferKeyEventL`.
//!
//! Every code here was probed by compiling `e32keys.h` and printing the enumerators
//! (experiment 76), not recalled. `ENonCharacterKeyBase = 0xf800` and every `EKey*`
//! below is `0xf800 + n`.
//!
//! A **softkey** does not arrive here. The button group container sits above this
//! view on the control stack (`ECoeStackPriorityCba` = 60 against the view's 0), turns
//! `EStdKeyDevice0`/`Device1` into a command and consumes the key, so the left and
//! right softkeys reach the menu and the shim (`crate::App::menu`) and never
//! [`crate::App::key`].
#![forbid(unsafe_code)]

use crate::abi::RawKeyEvent;

/// `TEventCode` (`w32std.h` line 266).
///
/// A character-producing key arrives as `Down`, then `Key` with `code` set, then `Up`.
/// Handle [`EventCode::Key`] and ignore the rest unless the application is a game:
/// `code` is 0 on the up and down events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCode {
    Null,
    Key,
    Up,
    Down,
    /// A `TEventCode` this crate has no name for. The window server has many more
    /// (pointer, focus, drag); a control that only wants keys ignores them.
    Other(i32),
}

impl EventCode {
    pub(crate) const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Null,
            1 => Self::Key,
            2 => Self::Up,
            3 => Self::Down,
            other => Self::Other(other),
        }
    }
}

/// One key event: `TKeyEvent` with names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// `TKeyCode` — the character or the `EKey*` constant. 0 on an up or down event.
    pub code: u32,
    /// `TStdScanCode` — the physical key, set on every event.
    pub scan_code: i32,
    /// `TEventModifier` (`e32keys.h`), e.g. `EModifierKeyUp = 0x0002_0000`.
    pub modifiers: u32,
    pub repeats: i32,
    /// Which of the three events this is.
    pub event: EventCode,
}

impl KeyEvent {
    pub(crate) const fn from_raw(raw: &RawKeyEvent, event: i32) -> Self {
        Self {
            code: raw.code,
            scan_code: raw.scan_code,
            modifiers: raw.modifiers,
            repeats: raw.repeats,
            event: EventCode::from_raw(event),
        }
    }

    /// True for the one event of the three that carries a character code.
    pub const fn is_press(&self) -> bool {
        matches!(self.event, EventCode::Key)
    }
}

/// `TKeyResponse` (`coedef.h` line 24). Returning [`KeyResponse::NotConsumed`] lets the
/// event travel further down the control stack, which is how the softkeys and the end
/// key keep working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyResponse {
    NotConsumed = 0,
    Consumed = 1,
}

/// `TKeyCode` values, as they appear in [`KeyEvent::code`].
pub mod key {
    /// `ENonCharacterKeyBase`.
    pub const NON_CHARACTER_BASE: u32 = 0xf800;
    /// `EKeyLeftArrow`.
    pub const LEFT: u32 = 0xf807;
    /// `EKeyRightArrow`.
    pub const RIGHT: u32 = 0xf808;
    /// `EKeyUpArrow`.
    pub const UP: u32 = 0xf809;
    /// `EKeyDownArrow`.
    pub const DOWN: u32 = 0xf80a;
    /// `EKeyMenu`.
    pub const MENU: u32 = 0xf836;
    /// `EKeyDevice0` — the left softkey.
    pub const SOFTKEY_LEFT: u32 = 0xf842;
    /// `EKeyDevice1` — the right softkey.
    pub const SOFTKEY_RIGHT: u32 = 0xf843;
    /// `EKeyDevice3` — the D-pad centre, "selection".
    pub const SELECT: u32 = 0xf845;
    /// `EKeyApplication0`.
    pub const APPLICATION0: u32 = 0xf852;
    /// `EKeyYes` — the green call key.
    pub const YES: u32 = 0xf862;
    /// `EKeyNo` — the red end key.
    pub const NO: u32 = 0xf863;
}

/// `TStdScanCode` values, as they appear in [`KeyEvent::scan_code`]. These are what
/// `docs/research/acceptance/emukey.py` drives, so a test that matches on a scan code
/// matches on what the emulator's keybind profile actually ships.
pub mod scan {
    /// `EStdKeyLeftArrow`.
    pub const LEFT: i32 = 0x0e;
    /// `EStdKeyRightArrow`.
    pub const RIGHT: i32 = 0x0f;
    /// `EStdKeyUpArrow`.
    pub const UP: i32 = 0x10;
    /// `EStdKeyDownArrow`.
    pub const DOWN: i32 = 0x11;
    /// `EStdKeyMenu`.
    pub const MENU: i32 = 0x94;
    /// `EStdKeyDevice0` — the left softkey.
    pub const SOFTKEY_LEFT: i32 = 0xa4;
    /// `EStdKeyDevice1` — the right softkey.
    pub const SOFTKEY_RIGHT: i32 = 0xa5;
    /// `EStdKeyDevice3` — the selection key.
    pub const SELECT: i32 = 0xa7;
    /// `EStdKeyApplication0`.
    pub const APPLICATION0: i32 = 0xb4;
    /// `EStdKeyYes`.
    pub const YES: i32 = 0xc4;
    /// `EStdKeyNo`.
    pub const NO: i32 = 0xc5;
}
