//! `std::io` for Symbian: the three traits, the error and the result type, with
//! `std`'s signatures and `std`'s documented semantics.
//!
//! A reader should not have to check whether ours differs, so the rules are the ones
//! `std` writes down: a short read or write is not an error, `Ok(0)` from `read` is end
//! of input, `Ok(0)` from `write` makes [`Write::write_all`] raise
//! [`ErrorKind::WriteZero`], and `read_exact`, `read_to_end` and `write_all` all retry
//! on [`ErrorKind::Interrupted`].
//!
//! # What is not here
//!
//! - `BufReader`, `BufWriter`, `Cursor`, `Lines`, `Bytes`, `Take`, `Chain`, `copy`,
//!   `stdin`/`stdout`/`stderr`. Nothing about them is hard; none of them is needed by
//!   step 71 and this SDK does not ship code it has not exercised.
//! - `Error::new(kind, source)` and `Error::other(e)`. An [`Error`] here is one Symbian
//!   `TInt` and has no room for a payload — see [`Error`].
//! - `read_to_string`. It needs `String::from_utf8`, which is a validity decision this
//!   crate has no opinion on yet; `read_to_end` plus `str::from_utf8` says the same
//!   thing without hiding it.
mod error;
mod kind;
mod read;
mod seek;
mod write;

pub use error::{Error, Result};
pub use kind::ErrorKind;
pub use read::Read;
pub use seek::{Seek, SeekFrom};
pub use write::Write;

/// `std::io::prelude`: the traits, and only the traits.
pub mod prelude {
    pub use super::{Read, Seek, Write};
}
