//! `SymbianError`: a system-wide `TInt` error code, with its `e32err.h` name.
use core::fmt;

use crate::ErrorKind;

/// A failed Symbian call, holding the exact `TInt` it returned.
///
/// The raw code is kept because it is the only thing a Symbian API and a Symbian user
/// agree on: component-specific codes below `-48` have no name in `e32err.h` and must
/// still survive a round trip through Rust.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbianError(i32);

/// The result of a Symbian call that can fail.
pub type Result<T> = core::result::Result<T, SymbianError>;

impl SymbianError {
    /// Wraps a `TInt` that has already been recognised as a failure.
    pub const fn from_code(code: i32) -> Self {
        Self(code)
    }

    /// The named code, for an error this crate raises itself.
    pub const fn of(kind: ErrorKind) -> Self {
        Self(kind.code())
    }

    /// The raw `TInt`.
    pub const fn code(self) -> i32 {
        self.0
    }

    /// The `KErrXxx` this code is (`e32err.h`).
    pub const fn kind(self) -> ErrorKind {
        ErrorKind::of(self.0)
    }
}

/// `KErrNone` is `0` and every failure is negative; a positive `TInt` is a result, not an
/// error, so only a negative code becomes an `Err`.
pub const fn check(code: i32) -> Result<i32> {
    if code < 0 {
        Err(SymbianError(code))
    } else {
        Ok(code)
    }
}

impl fmt::Debug for SymbianError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.kind().name(), self.0)
    }
}

impl fmt::Display for SymbianError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<ErrorKind> for SymbianError {
    fn from(kind: ErrorKind) -> Self {
        Self::of(kind)
    }
}
