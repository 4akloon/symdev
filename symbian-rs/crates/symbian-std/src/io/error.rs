//! `io::Error`: `std`'s shape over `symbian_core::SymbianError`.
use core::fmt;

use symbian_core::SymbianError;

use super::ErrorKind;

/// An I/O error, carrying the exact Symbian `TInt` that caused it.
///
/// The two accessors are `std`'s: [`Error::kind`] for a decision an application can
/// make portably, [`Error::raw_os_error`] for the code a Symbian programmer needs. It
/// is `Copy`, which `std::io::Error` is not — there is no boxed payload here to stop it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Error(SymbianError);

/// `std::io::Result`.
///
/// The error parameter has a default, which `std`'s alias does not: that is what lets
/// [`crate::prelude`] export the name without taking `Result<T, E>` away from a
/// program that globs it in. `Result<()>` and `Result<(), MyError>` both mean what
/// they look like.
pub type Result<T, E = Error> = core::result::Result<T, E>;

impl Error {
    /// Wraps a system error code, as `std::io::Error::from_raw_os_error` does.
    pub const fn from_raw_os_error(code: i32) -> Self {
        Self(SymbianError::from_code(code))
    }

    /// The `TInt` the Symbian call returned.
    ///
    /// Always `Some` here, where `std` returns `None` for an error it made up itself:
    /// every error in this crate started as a system code, including the ones the SDK
    /// raises before making a call (a path over `KMaxFileName` is `KErrOverflow`).
    pub const fn raw_os_error(&self) -> Option<i32> {
        Some(self.0.code())
    }

    /// The category, in `std`'s vocabulary.
    pub const fn kind(&self) -> ErrorKind {
        ErrorKind::of(self.0)
    }

    /// The underlying error, for code that wants the `e32err.h` name.
    pub const fn as_symbian(&self) -> SymbianError {
        self.0
    }
}

impl From<SymbianError> for Error {
    fn from(error: SymbianError) -> Self {
        Self(error)
    }
}

impl From<Error> for SymbianError {
    fn from(error: Error) -> Self {
        error.0
    }
}

/// `std::io::Error: From<ErrorKind>`, with the Symbian code the kind stands for.
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self(SymbianError::from_code(kind.code()))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({:?})", self.kind(), self.0)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl core::error::Error for Error {}

/// `fn main() -> io::Result<()>`: the process ends with the `TInt` the call failed
/// with, exactly as [`Error::raw_os_error`] reports it.
impl crate::IntoExitCode for Error {
    fn into_exit_code(self) -> crate::ExitCode {
        crate::IntoExitCode::into_exit_code(self.0)
    }
}
