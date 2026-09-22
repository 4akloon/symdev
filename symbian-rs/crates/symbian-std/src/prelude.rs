//! What `use symbian_std::prelude::*;` brings in: the I/O traits, as
//! `std::io::prelude` does, so `write_all`, `read_to_end` and `seek` are in scope.
//!
//! It also brings [`Result`], so that `#[symbian_std::main] fn main() -> Result<()>`
//! reads the way a Rust program reads. The alias has a defaulted error parameter
//! ([`crate::io::Result`]), so globbing the prelude does not take `Result<T, E>` away
//! from the rest of the file — the two-parameter form still means what it always did.
//!
//! And it brings [`write!`](symbian_fmt::write) and [`writeln!`](symbian_fmt::writeln),
//! which shadow `core`'s: the same syntax and, byte for byte, the same output, but a
//! plain `{}` of a string or an integer is appended directly instead of going through
//! `core::fmt` (experiment 99). A program that formats nothing else links none of
//! `core::fmt`; one that does gets `core::write!`'s exact behaviour for those pieces.
//!
//! It deliberately does not re-export `Vec`, `String` or `Box`: those come from
//! `alloc`, which an application already has, and shadowing them here would make it
//! harder, not easier, to see where a type comes from.
#[cfg(not(feature = "std"))]
pub use crate::io::{Read, Result, Seek, Write};
#[cfg(feature = "std")]
pub use std::io::{Read, Result, Seek, Write};
pub use symbian_fmt::{write, writeln};
