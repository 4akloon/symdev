//! `bafl.dso`'s resource file, as far as the shim needs Rust to hold one.

/// Storage for an `RResourceFile`: `TUint8 iImpl[KRscFileSize]` with `KRscFileSize = 24`
/// (`barsc.h` lines 57–60). The class has no other member and no virtual function, and
/// its only data is that byte array, so 24 bytes is its whole size; 4-byte alignment is
/// what the implementation object living in those bytes (`RResourceFileImpl`, pointers
/// and `TInt`s) needs, and what C++ gives the class by embedding it in word-aligned
/// objects. Rust never reads or writes the bytes: `symrs_rsc_open` constructs the object
/// in place with the exported constructor.
#[repr(C, align(4))]
pub struct RResourceFile {
    impl_: [u8; 24],
}

impl RResourceFile {
    /// Zeroed storage, ready for the shim to construct a file into.
    pub const fn zeroed() -> Self {
        Self { impl_: [0; 24] }
    }
}
