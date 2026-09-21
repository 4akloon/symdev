//! Size probe B: the same greeting through `locale!` with three languages declared.
#![no_std]

use core::fmt::Write;

use symbian_core::{Buf16, ErrorKind, Result, SymbianError, user};

symbian_std::locale! {
    languages: english, french, ukrainian;

    GREETING = { english: "Hello from Rust SDK", french: "Bonjour du SDK Rust", ukrainian: "Привіт від Rust SDK" },
}

#[symbian_std::main]
fn main() -> Result<()> {
    let greeting = GREETING.get();
    let mut note = Buf16::<64>::new();
    if write!(note, "{greeting} ({} chars)", greeting.len()).is_err() {
        return Err(SymbianError::of(ErrorKind::Overflow));
    }
    user::info_print(&note)?;
    user::after(5_000_000);
    Ok(())
}
