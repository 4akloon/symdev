//! `Math::Random()`, euser's pseudo-random `TUint32`.
//!
//! # What this is, and what it is not
//!
//! `Math::Random()` is exported by this ROM's euser (`000008b0 T _ZN4Math6RandomEv`)
//! and returns a `TUint32`. **What it is seeded from has never been observed** — it is
//! item 4 of the design spec's UNKNOWN list, and nothing on this host can settle it:
//! the ROM's euser is a binary, and the crypto DLLs that might hold a real entropy
//! source were never linked.
//!
//! So this module is honest about its two callers rather than claiming a quality it
//! cannot demonstrate:
//!
//! - [`fill_bytes`] backs `HashMap`'s seed and `std::random::random`, which need an
//!   unpredictable-enough value and not a cryptographic one. That is what euser offers.
//! - Anything that needs entropy for a key or a nonce must not use it. There is no
//!   `getrandom` on this platform, and pretending otherwise would be the kind of lie
//!   the first audit exposes.

pub fn fill_bytes(bytes: &mut [u8]) {
    let mut chunks = bytes.chunks_exact_mut(4);
    for chunk in &mut chunks {
        // SAFETY: `Math::Random` is a euser static member function (plain EABI, no
        // `this`) taking no argument and returning a `TUint32` by value.
        let word = unsafe { symbian_sys::sync::Math_Random() };
        chunk.copy_from_slice(&word.to_ne_bytes());
    }
    let tail = chunks.into_remainder();
    if !tail.is_empty() {
        // SAFETY: as above.
        let word = unsafe { symbian_sys::sync::Math_Random() };
        tail.copy_from_slice(&word.to_ne_bytes()[..tail.len()]);
    }
}
