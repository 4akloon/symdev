//! `ExitCode`: the one place a Rust value becomes the `TInt` `E32Main` returns.
use symbian_core::SymbianError;

/// The `TInt` `E32Main` returns to the loader.
pub struct ExitCode(pub i32);

impl ExitCode {
    pub fn from_main<T: IntoExitCode>(value: T) -> i32 {
        value.into_exit_code().0
    }
}

/// What a `fn main` may return.
///
/// Both entry points use it — `#[symbian_std::main]` and the older [`crate::entry!`] —
/// so there is one conversion from a Rust value to a process exit code and not two.
pub trait IntoExitCode {
    fn into_exit_code(self) -> ExitCode;
}

impl IntoExitCode for () {
    fn into_exit_code(self) -> ExitCode {
        ExitCode(0)
    }
}

impl IntoExitCode for i32 {
    fn into_exit_code(self) -> ExitCode {
        ExitCode(self)
    }
}

/// A failed Symbian call leaves the process with the code it failed with, so a `?` in
/// `main` reports the same `TInt` a C++ program would have returned.
impl IntoExitCode for SymbianError {
    fn into_exit_code(self) -> ExitCode {
        ExitCode(self.code())
    }
}

/// `fn main() -> Result<…>`: the success value's code, or the error's.
impl<T: IntoExitCode, E: IntoExitCode> IntoExitCode for Result<T, E> {
    fn into_exit_code(self) -> ExitCode {
        match self {
            Ok(value) => value.into_exit_code(),
            Err(error) => error.into_exit_code(),
        }
    }
}
