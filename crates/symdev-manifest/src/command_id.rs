//! `CommandId`: the number an application's menu item carries, derived from its name.
//!
//! A menu lives in two places that never see each other — the `.rss` symdev compiles
//! and the Rust source that matches on what `HandleCommandL` delivers — so the two
//! need an identifier that both can compute from the same text. A number in the
//! manifest would have to be repeated verbatim in the Rust source and would say
//! nothing; a name hashed the same way on both sides means the *word* appears in both
//! places and the number appears in neither.
//!
//! `symbian-rs/crates/symbian-ui/src/command.rs` holds the same function as a
//! `const fn`, and both sides carry the same fixed vectors in their tests. If one ever
//! drifts, those vectors fail rather than an application silently stopping reacting
//! to its own menu.
use std::fmt;

/// The command number for one menu item, as it goes into `MENU_ITEM { command = … }`.
///
/// The value is `0x4000 | (FNV-1a-32(name) & 0x3fff)`. Both ends of that range are
/// deliberate:
///
/// * **Below `0x4000`** is where every identifier the platform names lives:
///   `EEikCmdExit = 0x100` (`eikon.hrh:376`), Avkon's `EAknSoftkey*` at 3000–3200 and
///   `EAknCmd*` below that (`avkon.hrh`), and the reserved softkey ranges
///   `EAknSoftkeyLowestUser{Reject,Accept,Neutral}Id` at `0x1000`, `0x1100`, `0x1200`.
/// * **At or above `0x8000`** is where `CBA_BUTTON`'s `id` would stop being
///   representable: `eikon.rh`'s `CBA` declares it a `WORD`, so a softkey command has
///   16 bits and the sign of the sixteenth was never observed. Staying under `0x8000`
///   means one range serves a menu item and a softkey alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(u16);

impl CommandId {
    /// The id of the command named `name`.
    pub const fn of(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut hash: u32 = 0x811c_9dc5;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u32;
            hash = hash.wrapping_mul(0x0100_0193);
            i += 1;
        }
        Self(0x4000 | ((hash as u16) & 0x3fff))
    }

    /// The number, for the generated resource.
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:04x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::CommandId;

    /// The same table is asserted in `symbian-ui`'s `command::tests`. It is the only
    /// thing keeping the host's hash and the target's hash the same function.
    #[test]
    fn vectors_match_the_target_crate() {
        assert_eq!(CommandId::of("more").value(), 0x41e0);
        assert_eq!(CommandId::of("fewer").value(), 0x7612);
        assert_eq!(CommandId::of("reset").value(), 0x73c0);
        assert_eq!(CommandId::of("exit").value(), 0x5a85);
        assert_eq!(CommandId::of("").value(), 0x5dc5);
    }

    #[test]
    fn every_id_is_in_the_range_that_avoids_the_platform() {
        for name in ["", "a", "quit", "a much longer command name", "меню"] {
            let id = CommandId::of(name).value();
            assert!((0x4000..0x8000).contains(&id), "{name}: {id:#x}");
        }
    }

    #[test]
    fn different_names_are_different_commands() {
        assert_ne!(CommandId::of("more"), CommandId::of("fewer"));
    }
}
