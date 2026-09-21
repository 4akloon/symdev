//! `std::env::args` over `User::CommandLine`.
//!
//! Step 77 wrote this off as impossible and it is not. Symbian gives a process **one
//! string**, and two euser statics to read it with (`e32std.h` lines 4569-4570):
//! `User::CommandLineLength()` and `User::CommandLine(TDes16&)`. It is `User::`, not
//! `RProcess::` — this SDK's `euser.dso` exports no `RProcess::CommandLine` at all.
//!
//! # The splitting rule is ours, and it is the simplest one there is
//!
//! `RProcess::Create(aFileName, aCommand)` hands the child `aCommand` **verbatim**.
//! Nothing in `e32std.h`, `e32cmn.h` or `f32file.h` states a quoting convention, an
//! escape character or a separator, and no Symbian API anywhere in this SDK splits a
//! command line into a vector. So there is nothing to imitate and nothing to observe,
//! and inventing a quoting grammar here would be a guess that the first program to
//! pass a path with a space would find out about.
//!
//! The rule is therefore: **split on ASCII whitespace, and there is no quoting.** A
//! program that needs an argument with a space in it should agree its own encoding
//! with whatever starts it — which is what it would have to do on Symbian in C++ too.
//!
//! # What the first element is
//!
//! `std::env::args()` conventionally yields the program name first, and every argument
//! parser in the language skips element 0. Symbian's command line does **not** contain
//! it: the image and the command are separate arguments to `RProcess::Create`. So the
//! first element here is `RProcess().FileName()` — the running image's own path, e.g.
//! `E:\sys\bin\stdhello.exe` — read through the one shim its by-value `TFileName`
//! return needs. If that call fails, an empty first element is pushed anyway, so that
//! the index of every real argument stays the same whatever happens.

pub use super::common::Args;

use crate::ffi::OsString;
use crate::sys::pal::symbian::des::{PathBuf16, Utf16Buf};
use crate::vec::Vec;
use symbian_sys::euser::{User_CommandLine, User_CommandLineLength};
use symbian_sys::shim::symrs_process_file_name;

pub fn args() -> Args {
    let mut argv = Vec::new();
    argv.push(OsString::from(program_name().unwrap_or_default()));
    if let Some(line) = command_line() {
        argv.extend(line.split_ascii_whitespace().map(OsString::from));
    }
    Args::new(argv)
}

/// `RProcess().FileName()`, the running image's own path.
fn program_name() -> Option<crate::string::String> {
    let mut buf = PathBuf16::empty();
    // SAFETY: the shim takes a `TDes16*` and copies into it under the descriptor's own
    // `MaxLength`, answering `KErrOverflow` rather than overflowing; `buf` is a real
    // `TBuf16<256>` of the layout experiment 69 observed, alive across the call. The
    // shim cannot leave: `RProcess::FileName` allocates nothing.
    let code = unsafe { symrs_process_file_name(buf.as_tdes16()) };
    if code != 0 {
        return None;
    }
    buf.to_utf8()
}

/// The whole command line as one string, or `None` when there is none.
fn command_line() -> Option<crate::string::String> {
    // SAFETY: a euser static with no arguments, reading this process's own creation
    // record. It cannot fail and it cannot leave.
    let len = unsafe { User_CommandLineLength() };
    if len <= 0 {
        return None;
    }
    let mut buf = Utf16Buf::with_capacity(len as usize).ok()?;
    // SAFETY: a euser static taking one `TDes16&`, which is the `TPtr16` euser itself
    // built over `buf`'s heap storage with `iMaxLength` equal to the length just asked
    // for — so the copy cannot overflow, which matters because an overflow is `USER 11`
    // and a panic no `TRAP` catches.
    unsafe { User_CommandLine(buf.as_tdes16()) };
    buf.to_utf8()
}
