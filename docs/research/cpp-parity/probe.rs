//! The Rust half of the heap/startup probe (`docs/research/cpp-parity.md`).
//!
//! It is one file, pulled into an example with
//!
//! ```ignore
//! #[path = "../../../docs/research/cpp-parity/probe.rs"]
//! mod probe;
//! ```
//!
//! so that the four examples share it and nothing under `symbian-rs/crates` has to
//! change. Everything here is declared locally against euser's own exports, taken
//! from `nm -D epoc32/release/armv5/lib/euser.dso`:
//!
//! * `00000a5c T _ZN4User9AllocSizeERi` — `User::AllocSize(TInt&)`. e32std.h line 4484.
//!   It **returns the number of allocated cells** and writes the total number of bytes
//!   across those cells into its argument. That is the "bytes allocated" figure; it is
//!   not `RHeap::Size()`, which e32cmn.inl line 78 defines as "the total number of
//!   bytes committed by the host chunk" and which therefore moves in page-sized steps.
//! * `000020d8 T _ZN4User10NTickCountEv` — `User::NTickCount()`. A free-running
//!   `TUint32`. **No header in this SDK states its period**; the C++ half of the probe
//!   asks `UserHal::TickPeriod` for the *system* tick period, which is the only rate
//!   the platform will state, and the note converts with that plus the 1 000 µs
//!   nanotick period `symbian-core/src/time.rs` records as measured inside EKA2L1.
//!
//! The probe is deliberately dumb: two euser calls and a subtraction, the same two on
//! both sides, so that what it adds to each image is the same shape.
#![allow(dead_code)]

unsafe extern "C" {
    #[link_name = "_ZN4User9AllocSizeERi"]
    fn User_AllocSize(total: *mut i32) -> i32;
    #[link_name = "_ZN4User10NTickCountEv"]
    fn User_NTickCount() -> u32;
}

/// One reading of the thread's allocator and of the nanokernel tick counter.
#[derive(Clone, Copy)]
pub struct Probe {
    /// `User::AllocSize`'s return value: how many cells are allocated.
    pub cells: i32,
    /// `User::AllocSize`'s out-parameter: the bytes across those cells.
    pub bytes: i32,
    /// `User::NTickCount()`.
    pub ticks: u32,
}

impl Probe {
    pub fn now() -> Self {
        let mut bytes: i32 = 0;
        // SAFETY: both are euser static member functions (plain EABI, no `this`) that
        // cannot leave; `&mut bytes` is a valid exclusively borrowed `i32` for the
        // whole of the first call.
        let cells = unsafe { User_AllocSize(&mut bytes) };
        let ticks = unsafe { User_NTickCount() };
        Self {
            cells,
            bytes,
            ticks,
        }
    }

    pub fn ticks_since(self, earlier: Self) -> u32 {
        self.ticks.wrapping_sub(earlier.ticks)
    }
}
