//! `write!` and `writeln!` with `core`'s syntax and `core`'s output, which append a
//! plain `{}` of a string or an integer directly instead of formatting it through
//! `core::fmt` (experiment 101).
//!
//! An application imports them by name, `use symbian_std::{write, writeln};`, which
//! shadows `core`'s; nothing else in the program changes. (Not through the prelude's
//! glob: a glob-imported `write` is ambiguous with `core`'s, rustc E0659.) They exist
//! for size: `core::fmt::write`, the `Formatter` and the integer `Display` are about
//! 1.3 kB of every image that formats anything, and C++ pays nothing for the same
//! work because `TDes::Format`/`AppendNum` are in ROM.
//!
//! # What is fast, and what is exactly `write!`
//!
//! The macro takes the format string apart at compile time when every hole is a
//! plain `{}`, `{0}` or `{name}` (inline captures included). Each piece is then
//! written by the most direct path its *types* allow ([`__private::Probe`]):
//!
//! - into a destination with a native append (`symbian_core::Buf16`): `push_str`, and
//!   euser's `TDes16::AppendNum` for an integer, which is code in ROM;
//! - into any other `fmt::Write` (`String`, a `Formatter`, a user's type): the same
//!   `write_str`/`write_char` calls `core::fmt` would make, with the digits produced
//!   here;
//! - anything else — `bool`, a float, a 128-bit integer, a user's `Display`, an
//!   `io::Write` destination — through `write_fmt(format_args!("{}", a))`, one piece
//!   at a time, which is `core::fmt` exactly.
//!
//! An invocation with any format spec (`{:5}`, `{:x}`, `{:?}`, `{:.2}`…), a format
//! string that is not a string literal, or anything `format_args!` would reject is
//! passed to `core::write!` whole, so its behaviour and its diagnostics are
//! `core`'s own.
//!
//! The guarantee, tested on the host against `core::write!` call for call: the
//! destination sees the same `write_str`/`write_char` calls with the same text, in the
//! same order, and the result is the same — so the output is byte-identical for every
//! destination, including one that fails part-way.
#![no_std]

extern crate alloc;

mod arg;
mod decimal;
mod probe;
mod sink;

/// `core::write!`, with plain `{}` of strings and integers appended directly. See
/// the crate documentation for what is fast and what is `core::write!` exactly.
#[macro_export]
macro_rules! write {
    ($dst:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::__private::write_pieces!($crate; false; $dst; $fmt; $($arg),*)
    };
    ($($any:tt)*) => {
        ::core::write!($($any)*)
    };
}

/// `core::writeln!`, as [`write!`] is `core::write!`.
#[macro_export]
macro_rules! writeln {
    ($dst:expr $(,)?) => {
        $crate::__private::write_pieces!($crate; true; $dst; ""; )
    };
    ($dst:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::__private::write_pieces!($crate; true; $dst; $fmt; $($arg),*)
    };
    ($($any:tt)*) => {
        ::core::writeln!($($any)*)
    };
}

/// What the expansion names. Not an API.
#[doc(hidden)]
pub mod __private {
    pub use crate::decimal::Decimal;
    pub use crate::probe::{
        Enter, Probe, SinkKind, SinkTag, SlowKind, SlowTag, WriteKind, WriteTag,
    };
    pub use crate::sink::Sink;
    pub use symbian_macros::__write_pieces as write_pieces;
}

pub use sink::Sink;
