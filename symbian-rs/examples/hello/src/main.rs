//! Hello (experiment 69): the note is built with `write!` into a stack descriptor and
//! shown through a safe wrapper. No `unsafe` block, no raw C function, no `_LIT` static —
//! `examples/hello-raw` keeps the pre-`symbian-core` version for comparison.
//!
//! The entry point is `#[symbian_std::main]` (experiment 81): an ordinary `fn main`
//! returning a `Result`, with the `E32Main()` `eexe.lib` calls written by the
//! attribute and the error's `TInt` becoming the exit code. This file is also what
//! `symdev new --language rust` writes.
#![no_std]

use core::fmt::Write;

use symbian_core::{Buf16, ErrorKind, Result, SymbianError, user};

const GREETING: &str = "Hello from Rust SDK";

#[symbian_std::main]
fn main() -> Result<()> {
    let mut note = Buf16::<64>::new();
    if write!(note, "{GREETING} ({} chars)", GREETING.len()).is_err() {
        return Err(SymbianError::of(ErrorKind::Overflow));
    }
    user::info_print(&note)?;
    user::after(5_000_000);
    Ok(())
}
