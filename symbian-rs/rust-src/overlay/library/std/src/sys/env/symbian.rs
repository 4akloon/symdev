//! `std::env`'s variables on Symbian: there are none, deliberately and permanently.
//!
//! **Symbian OS 9.3 has no environment.** There is no `environ`, no `getenv`, no
//! inherited block of `KEY=VALUE` pairs anywhere in `e32std.h`, and `RProcess::Create`
//! takes a file name and one command-line descriptor and nothing else. A process's
//! whole inheritance from its creator is that one string and the handles passed with
//! `RProcess::SetParameter`.
//!
//! # Why this is not backed by a process-local map
//!
//! It would be easy to keep a `HashMap` in the process and let `set_var` and `var`
//! agree with each other. That is the tempting answer and it is a lie, and the first
//! child process exposes it: on every other platform `env::set_var` is inherited, so a
//! program that sets `RUST_LOG` and spawns a helper expects the helper to see it. Here
//! the helper would see nothing, and the failure would be silent and remote from its
//! cause. Better that `var` says "not there" from the start, which is true.
//!
//! # What is different from `sys::env::unsupported`
//!
//! One thing, and it matters: [`env`] returns an **empty iterator** rather than
//! panicking. "This platform has no environment variables" is a fact, and the honest
//! spelling of it is an empty list, not killing the process for asking.

use crate::ffi::{OsStr, OsString};
use crate::{fmt, io};

/// The environment as an iterator: always empty, because there is never anything in it.
pub struct Env(());

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().finish()
    }
}

impl Iterator for Env {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<(OsString, OsString)> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

pub fn env() -> Env {
    Env(())
}

pub fn getenv(_: &OsStr) -> Option<OsString> {
    None
}

pub unsafe fn setenv(_: &OsStr, _: &OsStr) -> io::Result<()> {
    Err(io::const_error!(
        io::ErrorKind::Unsupported,
        "Symbian has no environment variables: a process inherits one command line and nothing else"
    ))
}

pub unsafe fn unsetenv(_: &OsStr) -> io::Result<()> {
    Err(io::const_error!(
        io::ErrorKind::Unsupported,
        "Symbian has no environment variables: a process inherits one command line and nothing else"
    ))
}
