//! The per-thread trap handler and cleanup stack, as `std` reaches it.
//!
//! The type itself is [`symbian_sys::cleanup::TrapCleanup`], and it is there rather
//! than here for one reason: the `no_std` runtime needs exactly the same thing, and a
//! second copy is how the two paths would drift apart. A `no_std` console application
//! without one dies on the first `CleanupStack::PushL` below it — measured, panic
//! `E32USER-CBase 69` — which is the asymmetry this re-export exists to end.
//!
//! `std::os::symbian::start` **is** this program's `E32Main` body and `sys::thread`'s
//! trampoline is its thread entry, so both install one, exactly as a C++ `E32Main`
//! conventionally opens with `CTrapCleanup::New()`.

pub use symbian_sys::cleanup::TrapCleanup;
