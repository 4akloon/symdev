//! `io::ErrorKind`: `std`'s vocabulary, and the `e32err.h` code behind each word.
use symbian_core::{ErrorKind as Sym, SymbianError};

/// A category of I/O error, with `std::io::ErrorKind`'s names and meanings.
///
/// Only the variants a Symbian call can actually produce are here. The set is
/// `#[non_exhaustive]`, as `std`'s is, so matching needs a `_` arm and a later
/// subsystem can add its own without breaking a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// `KErrNotFound`, `KErrPathNotFound`.
    NotFound,
    /// `KErrAccessDenied`, `KErrPermissionDenied`.
    PermissionDenied,
    /// `KErrAlreadyExists`.
    AlreadyExists,
    /// `KErrArgument`, `KErrBadHandle`, `KErrUnderflow`.
    InvalidInput,
    /// `KErrCorrupt`, `KErrBadDescriptor`, `KErrTotalLossOfPrecision`.
    InvalidData,
    /// `KErrBadName`: a path the file server will not parse.
    InvalidFilename,
    /// `KErrEof`: the file ended before the bytes a caller demanded.
    UnexpectedEof,
    /// `KErrOverflow`, `KErrTooBig`: more bytes than the destination can hold.
    FileTooLarge,
    /// `KErrDiskFull`, `KErrDirFull`.
    StorageFull,
    /// `KErrInUse`, `KErrLocked`, `KErrServerBusy`.
    ResourceBusy,
    /// `KErrNoMemory`.
    OutOfMemory,
    /// `KErrNotSupported`, `KErrExtensionNotSupported`.
    Unsupported,
    /// `KErrTimedOut`.
    TimedOut,
    /// A write that accepted no bytes. Nothing in the file API produces it — Symbian
    /// writes the whole descriptor or fails — but [`crate::io::Write::write_all`] is
    /// documented to raise it, so the name exists.
    WriteZero,
    /// A call that should be retried. No Symbian file call produces one; the variant is
    /// here because `read_exact`, `read_to_end` and `write_all` are documented to
    /// retry on it and a later subsystem may raise it.
    Interrupted,
    /// Everything `e32err.h` names that has no `std` word, and everything it does not
    /// name at all. The exact code is still in
    /// [`crate::io::Error::raw_os_error`].
    Other,
}

impl ErrorKind {
    /// The kind `std` would give the error a Symbian call reported.
    ///
    /// Where `e32err.h` and `std` mean the same thing the mapping is exact; where they
    /// do not, the code falls to [`ErrorKind::Other`] rather than being forced into a
    /// word that would mislead. Nothing is lost either way — the `TInt` is still there.
    pub const fn of(error: SymbianError) -> Self {
        match error.kind() {
            Sym::NotFound | Sym::PathNotFound => Self::NotFound,
            Sym::AccessDenied | Sym::PermissionDenied => Self::PermissionDenied,
            Sym::AlreadyExists => Self::AlreadyExists,
            Sym::Argument | Sym::BadHandle | Sym::Underflow => Self::InvalidInput,
            Sym::Corrupt | Sym::BadDescriptor | Sym::TotalLossOfPrecision => Self::InvalidData,
            Sym::BadName => Self::InvalidFilename,
            Sym::Eof => Self::UnexpectedEof,
            Sym::Overflow | Sym::TooBig => Self::FileTooLarge,
            Sym::DiskFull | Sym::DirFull => Self::StorageFull,
            Sym::InUse | Sym::Locked | Sym::ServerBusy => Self::ResourceBusy,
            Sym::NoMemory => Self::OutOfMemory,
            Sym::NotSupported | Sym::ExtensionNotSupported => Self::Unsupported,
            Sym::TimedOut => Self::TimedOut,
            _ => Self::Other,
        }
    }

    /// The Symbian code this crate raises for a kind of its own making.
    ///
    /// Only the kinds `symbian-std` itself produces have one; the rest keep
    /// `KErrGeneral`, because inventing a code for them would put a number in
    /// `raw_os_error` that no Symbian call ever returned.
    pub(crate) const fn code(self) -> i32 {
        match self {
            Self::UnexpectedEof => Sym::Eof.code(),
            Self::FileTooLarge => Sym::TooBig.code(),
            Self::WriteZero => Sym::Write.code(),
            Self::InvalidInput => Sym::Argument.code(),
            Self::OutOfMemory => Sym::NoMemory.code(),
            Self::ResourceBusy => Sym::InUse.code(),
            _ => Sym::General.code(),
        }
    }
}
