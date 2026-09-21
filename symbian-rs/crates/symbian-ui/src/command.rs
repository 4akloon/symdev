//! `Command`: what the Options menu and the softkeys send the application.
//!
//! The menu itself is declared in `symdev.toml`, because it is a compiled resource
//! that has to exist before any Rust runs. What crosses into Rust is a number, and a
//! number in a `match` arm says nothing — so both sides derive it from the *same
//! word*: `[[ui.menu]] id = "more"` in the manifest and [`Command::named`]`("more")`
//! here compute the identical value, and the number appears in neither place.
//!
//! ```ignore
//! const MORE: Command = Command::named("more");
//!
//! fn command(&mut self, command: Command, ui: &Ui) -> Result<()> {
//!     match command {
//!         MORE => self.bars += 1,
//!         _ => return Ok(()),
//!     }
//!     ui.redraw();
//!     Ok(())
//! }
//! ```
#![forbid(unsafe_code)]

/// One command, as `CEikAppUi::HandleCommandL` was handed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Command(i32);

impl Command {
    /// `EEikCmdExit` (`eikon.hrh`), what the right softkey carries.
    ///
    /// An application never sees this one: the shim ends the application on it and
    /// does not forward it. It is named so that the value is written down once.
    pub const EXIT: Self = Self(0x100);

    /// `EAknSoftkeyOptions` (`avkon.hrh`), what the left softkey carries when there
    /// is a menu.
    ///
    /// An application does not see this one either — the framework itself watches for
    /// it and opens the menu bar instead of passing it on.
    pub const OPTIONS: Self = Self(3000);

    /// The command the menu item named `name` sends.
    ///
    /// The value is `0x4000 | (FNV-1a-32(name) & 0x3fff)`, the same function
    /// `symdev-manifest`'s `CommandId` applies to `[[ui.menu]] id`. The range sits
    /// above every identifier the platform names (`EEikCmd*` at `0x100`, Avkon's
    /// `EAknSoftkey*` at 3000–3200, its reserved softkey ranges at `0x1000`–`0x12ff`)
    /// and below `0x8000`, where `CBA_BUTTON`'s `WORD` id would run out.
    pub const fn named(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut hash: u32 = 0x811c_9dc5;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u32;
            hash = hash.wrapping_mul(0x0100_0193);
            i += 1;
        }
        Self((0x4000 | ((hash as u16) & 0x3fff)) as i32)
    }

    /// True when this is the command the menu item named `name` sends.
    ///
    /// The same test as `command == Command::named(name)`, for the applications that
    /// would rather write the word in the condition than declare a constant.
    pub const fn is(self, name: &str) -> bool {
        self.0 == Self::named(name).0
    }

    /// The raw `TInt` the framework used, for an application that has to log it.
    pub const fn raw(self) -> i32 {
        self.0
    }

    pub(crate) const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }
}

/// The same table is asserted in `symdev-manifest`'s `command_id::tests`, and it is
/// the only thing keeping the host's hash and this one the same function: if they ever
/// drift, an application silently stops reacting to its own menu. These are `const`
/// assertions rather than `#[test]`s because this workspace builds for the phone and
/// `cargo test` never runs in it — a compile failure is the only failure it can have.
const _: () = {
    assert!(Command::named("more").raw() == 0x41e0);
    assert!(Command::named("fewer").raw() == 0x7612);
    assert!(Command::named("reset").raw() == 0x73c0);
    assert!(Command::named("exit").raw() == 0x5a85);
    assert!(Command::named("").raw() == 0x5dc5);
    // Above everything the platform names and below `CBA_BUTTON`'s sixteenth bit.
    assert!(Command::named("quit").raw() >= 0x4000);
    assert!(Command::named("quit").raw() < 0x8000);
    assert!(Command::named("quit").raw() != Command::EXIT.raw());
    assert!(Command::named("quit").raw() != Command::OPTIONS.raw());
    assert!(Command::named("more").is("more"));
    assert!(!Command::named("more").is("fewer"));
};
