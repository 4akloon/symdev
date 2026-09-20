//! `E32Target`: what elf2e32 is asked to produce.

/// What elf2e32 is asked to produce (`--targettype`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E32Target {
    Exe,
    Dll,
}
