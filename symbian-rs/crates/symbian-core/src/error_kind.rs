//! `ErrorKind`: the `KErrXxx` names of `epoc32/include/e32err.h`.

/// Declares the system-wide error codes once, and derives the enum, the lookup and the
/// name from that one list. Every line is a `const TInt KErrXxx` of `e32err.h`, in the
/// order the header declares them.
macro_rules! error_kinds {
    ($($code:literal => $variant:ident, $name:literal;)+) => {
        /// A system-wide Symbian error code, by name (`e32err.h`).
        ///
        /// `Unknown` carries a code the header does not name: a component-specific error
        /// (below `-48`) or a positive value, which is never an error.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum ErrorKind {
            $($variant,)+
            Unknown(i32),
        }

        impl ErrorKind {
            /// The kind `e32err.h` gives this `TInt`.
            pub const fn of(code: i32) -> Self {
                match code {
                    $($code => Self::$variant,)+
                    other => Self::Unknown(other),
                }
            }

            /// The `KErrXxx` identifier, as written in `e32err.h`.
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                    Self::Unknown(_) => "KErrUnnamed",
                }
            }

            /// The `TInt` this kind stands for, when the header names one.
            pub const fn code(self) -> i32 {
                match self {
                    $(Self::$variant => $code,)+
                    Self::Unknown(code) => code,
                }
            }
        }
    };
}

error_kinds! {
    0 => None, "KErrNone";
    -1 => NotFound, "KErrNotFound";
    -2 => General, "KErrGeneral";
    -3 => Cancel, "KErrCancel";
    -4 => NoMemory, "KErrNoMemory";
    -5 => NotSupported, "KErrNotSupported";
    -6 => Argument, "KErrArgument";
    -7 => TotalLossOfPrecision, "KErrTotalLossOfPrecision";
    -8 => BadHandle, "KErrBadHandle";
    -9 => Overflow, "KErrOverflow";
    -10 => Underflow, "KErrUnderflow";
    -11 => AlreadyExists, "KErrAlreadyExists";
    -12 => PathNotFound, "KErrPathNotFound";
    -13 => Died, "KErrDied";
    -14 => InUse, "KErrInUse";
    -15 => ServerTerminated, "KErrServerTerminated";
    -16 => ServerBusy, "KErrServerBusy";
    -17 => Completion, "KErrCompletion";
    -18 => NotReady, "KErrNotReady";
    -19 => Unknown_, "KErrUnknown";
    -20 => Corrupt, "KErrCorrupt";
    -21 => AccessDenied, "KErrAccessDenied";
    -22 => Locked, "KErrLocked";
    -23 => Write, "KErrWrite";
    -24 => DisMounted, "KErrDisMounted";
    -25 => Eof, "KErrEof";
    -26 => DiskFull, "KErrDiskFull";
    -27 => BadDriver, "KErrBadDriver";
    -28 => BadName, "KErrBadName";
    -29 => CommsLineFail, "KErrCommsLineFail";
    -30 => CommsFrame, "KErrCommsFrame";
    -31 => CommsOverrun, "KErrCommsOverrun";
    -32 => CommsParity, "KErrCommsParity";
    -33 => TimedOut, "KErrTimedOut";
    -34 => CouldNotConnect, "KErrCouldNotConnect";
    -35 => CouldNotDisconnect, "KErrCouldNotDisconnect";
    -36 => Disconnected, "KErrDisconnected";
    -37 => BadLibraryEntryPoint, "KErrBadLibraryEntryPoint";
    -38 => BadDescriptor, "KErrBadDescriptor";
    -39 => Abort, "KErrAbort";
    -40 => TooBig, "KErrTooBig";
    -41 => DivideByZero, "KErrDivideByZero";
    -42 => BadPower, "KErrBadPower";
    -43 => DirFull, "KErrDirFull";
    -44 => HardwareNotAvailable, "KErrHardwareNotAvailable";
    -45 => SessionClosed, "KErrSessionClosed";
    -46 => PermissionDenied, "KErrPermissionDenied";
    -47 => ExtensionNotSupported, "KErrExtensionNotSupported";
    -48 => CommsBreak, "KErrCommsBreak";
}
