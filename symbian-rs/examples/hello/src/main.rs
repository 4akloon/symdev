//! Hello (experiment 69): the note is built with `write!` into a stack descriptor and
//! shown through a safe wrapper. No `unsafe` block, no raw C function, no `_LIT` static —
//! `examples/hello-raw` keeps the pre-`symbian-core` version for comparison.
#![no_std]
#![no_main]

use core::fmt::Write;

use symbian_core::{Buf16, ErrorKind, Result, SymbianError, user};

const GREETING: &str = "Hello from Rust SDK";

fn run() -> Result<()> {
    let mut note = Buf16::<64>::new();
    if write!(note, "{GREETING} ({} chars)", GREETING.len()).is_err() {
        return Err(SymbianError::of(ErrorKind::Overflow));
    }
    user::info_print(&note)?;
    user::after(5_000_000);
    Ok(())
}

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

symbian_runtime::entry!(main);
