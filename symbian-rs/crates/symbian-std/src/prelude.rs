//! What `use symbian_std::prelude::*;` brings in: the I/O traits, as
//! `std::io::prelude` does, so `write_all`, `read_to_end` and `seek` are in scope.
//!
//! It deliberately does not re-export `Vec`, `String` or `Box`: those come from
//! `alloc`, which an application already has, and shadowing them here would make it
//! harder, not easier, to see where a type comes from.
pub use crate::io::{Read, Seek, Write};
