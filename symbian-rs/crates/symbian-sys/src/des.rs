//! The one descriptor shape observed so far: a `_LIT16` literal.
//!
//! `e32des16.h`: a descriptor header is one `TUint` whose top nibble is the type
//! (`KShiftDesType16 = 28`) and whose low 28 bits are the length (`KMaskDesLength16 =
//! 0xfffffff`). The type enum itself is not in the public headers, but experiment 65a
//! compiled `_LIT(KHello, "Hi")` with the observed GCCE argv and read the bytes:
//! `02000000 48006900 0000` — length 2 with type nibble 0, then the UTF-16 code units in
//! place. euser reads such a static through `const TDesC16&`, so a `#[repr(C)]` struct of
//! that layout is a valid argument for every `const TDesC16&` parameter.

/// A UTF-16 literal descriptor (`_LIT16`): the type/length word, then `N` code units.
#[repr(C)]
pub struct Lit16<const N: usize> {
    type_length: u32,
    buf: [u16; N],
}

/// The opaque `TDesC16` an euser export takes by `const&`; only ever seen behind a pointer.
#[repr(C)]
pub struct TDesC16 {
    _private: [u8; 0],
}

/// `KMaskDesLength16` (`e32des16.h`): the low 28 bits of a descriptor's header word are
/// its length in code units, and the top nibble is the type.
pub const KMASK_DES_LENGTH_16: u32 = 0x0fff_ffff;

unsafe extern "C" {
    /// `00001c38 T _ZNK7TDesC163PtrEv` — `TDesC16::Ptr() const`: where the code units
    /// are.
    ///
    /// It is an exported function and not a field read because the answer depends on
    /// the type nibble — a `TPtrC16` keeps a pointer where a `TBufC16` keeps the units
    /// themselves, and an `EBufCPtr` keeps neither. Calling euser is how this SDK reads
    /// a descriptor it did not build, instead of assuming which of the five shapes it
    /// is.
    #[link_name = "_ZNK7TDesC163PtrEv"]
    pub fn TDesC16_Ptr(this: *const TDesC16) -> *const u16;
}

impl<const N: usize> Lit16<N> {
    /// Builds the literal from ASCII bytes at compile time (`static HELLO: Lit16<5> =
    /// Lit16::ascii(b"Hello")`). Non-ASCII input is refused: the observed layout holds
    /// UTF-16 code units and this constructor does no transcoding.
    pub const fn ascii(s: &[u8; N]) -> Self {
        let mut buf = [0u16; N];
        let mut i = 0;
        while i < N {
            assert!(s[i] < 0x80, "Lit16::ascii: byte outside ASCII");
            buf[i] = s[i] as u16;
            i += 1;
        }
        // Type nibble 0 (as observed), length in the low 28 bits.
        Self {
            type_length: N as u32,
            buf,
        }
    }

    /// The `const TDesC16&` euser expects.
    pub const fn as_desc(&self) -> *const TDesC16 {
        (self as *const Self).cast()
    }

    /// The code units, for host-side tests of the layout.
    pub const fn units(&self) -> &[u16; N] {
        &self.buf
    }

    /// The header word, for host-side tests of the layout.
    pub const fn header(&self) -> u32 {
        self.type_length
    }
}
