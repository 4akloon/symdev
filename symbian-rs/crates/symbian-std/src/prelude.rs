//! What `use symbian_std::prelude::*;` brings in: the I/O traits, as
//! `std::io::prelude` does, so `write_all`, `read_to_end` and `seek` are in scope.
//!
//! It also brings [`Result`], so that `#[symbian_std::main] fn main() -> Result<()>`
//! reads the way a Rust program reads. The alias has a defaulted error parameter
//! ([`crate::io::Result`]), so globbing the prelude does not take `Result<T, E>` away
//! from the rest of the file — the two-parameter form still means what it always did.
//!
//! It does **not** bring the fast [`crate::write!`] and [`crate::writeln!`], although
//! that was the plan: a macro named `write` that arrives through a glob import is
//! ambiguous with `core`'s (rustc E0659, experiment 101), so every program that globbed
//! this prelude and called `write!` would stop compiling. They are imported by name:
//! `use symbian_std::{write, writeln};`.
//!
//! It deliberately does not re-export `Vec`, `String` or `Box`: those come from
//! `alloc`, which an application already has, and shadowing them here would make it
//! harder, not easier, to see where a type comes from.
#[cfg(not(feature = "std"))]
pub use crate::io::{Read, Result, Seek, Write};
#[cfg(feature = "std")]
pub use std::io::{Read, Result, Seek, Write};
