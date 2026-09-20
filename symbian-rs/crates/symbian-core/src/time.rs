//! The Symbian clocks, safely: three counters that count up from an unspecified
//! origin, and the wall clock (design spec §7).
//!
//! This is the Symbian-shaped layer — the escape hatch of §6a. What an application
//! writes is `symbian_std::time::{Instant, SystemTime}`, which is built out of these.
//!
//! Each counter is a distinct type rather than a free function, because the one thing
//! that matters about them is that they are **not interchangeable**: they have
//! different periods, different origins and, for [`FastCounter`], a direction that is
//! a property of the board.
use symbian_sys::time::{
    TTime, TTime_HomeTime, TTime_UniversalTime, User_FastCounter, User_NTickCount,
    User_TickCount, UserHal_TickPeriod,
};

use crate::error::{Result, check};

/// The **nanokernel** tick counter, `User::NTickCount()`.
///
/// A free-running `TUint32` that counts up and wraps at 2^32 ticks; nothing resets it
/// while the machine is up, and a device clock change does not touch it. It is the
/// counter `symbian_std::time::Instant` is built on.
///
/// The header states no period. `HALData::ENanoTickPeriod` would, but it lives in
/// `hal.dll`, which is not on this SDK's link line, so the period is **measured**
/// instead (experiment 85) and must be measured again on a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NanoTicks(u32);

impl NanoTicks {
    /// Reads the counter.
    pub fn now() -> Self {
        // SAFETY: a euser static member function (plain EABI, no `this`) that takes
        // nothing, returns a `TUint32` by value in r0, touches no memory of ours and
        // cannot leave.
        Self(unsafe { User_NTickCount() })
    }

    /// The raw tick value.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Ticks from `earlier` to `self`, counted the only way a wrapping counter can be:
    /// modulo 2^32. Correct for any true interval shorter than one wrap, and silently
    /// aliased for anything longer — there is no bit left to tell the two apart.
    pub const fn ticks_since(self, earlier: Self) -> u32 {
        self.0.wrapping_sub(earlier.0)
    }

    /// The tick `ticks` after this one.
    pub const fn advanced_by(self, ticks: u32) -> Self {
        Self(self.0.wrapping_add(ticks))
    }
}

/// The **system** tick counter, `User::TickCount()`, whose period
/// [`SystemTicks::period_micros`] reports.
///
/// Coarser than [`NanoTicks`] — it is the tick the kernel's timer queues run on — and
/// also a `TUint` that wraps at 2^32.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTicks(u32);

impl SystemTicks {
    /// Reads the counter.
    pub fn now() -> Self {
        // SAFETY: as `NanoTicks::now` — a nullary euser static member returning a
        // scalar, with no way to leave.
        Self(unsafe { User_TickCount() })
    }

    /// The raw tick value.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Ticks from `earlier` to `self`, modulo 2^32.
    pub const fn ticks_since(self, earlier: Self) -> u32 {
        self.0.wrapping_sub(earlier.0)
    }

    /// The period of this counter in microseconds (`UserHal::TickPeriod`), which is
    /// the one clock rate the platform will state rather than make us measure.
    pub fn period_micros() -> Result<i32> {
        let mut period: i32 = 0;
        // SAFETY: `UserHal::TickPeriod` is a euser static member taking a reference to
        // a `TTimeIntervalMicroSeconds32`, a 4-byte class holding one `TInt`; `&mut
        // period` is a valid, aligned, exclusively borrowed `i32` for the whole call.
        // The function writes it and returns a `TInt`, and cannot leave.
        let code = unsafe { UserHal_TickPeriod(&mut period) };
        check(code)?;
        Ok(period)
    }
}

/// `User::FastCounter()`: the highest-resolution counter the board offers.
///
/// Neither its frequency (`HALData::EFastCounterFrequency`) nor its **direction**
/// (`HALData::EFastCounterCountsUp` — it may count down) can be read without
/// `hal.dll`, which is not on this SDK's link line. Nothing in this SDK builds a
/// duration out of it for that reason; it is here because a program that knows its own
/// board may want it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FastCounter(u32);

impl FastCounter {
    /// Reads the counter.
    pub fn now() -> Self {
        // SAFETY: as `NanoTicks::now`.
        Self(unsafe { User_FastCounter() })
    }

    /// The raw counter value.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A Symbian `TTime`: microseconds since midnight, 1 January 0 AD nominal Gregorian
/// (`e32std.h` line 1770), as a signed 64-bit count. BC dates are negative.
///
/// This is the wall clock. The user can set it, a network can set it, and it can
/// therefore go backwards — see [`Ttime::universal`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ttime(TTime);

impl Ttime {
    /// The current **UTC** time (`TTime::UniversalTime`).
    ///
    /// This is the one to build a `SystemTime` from: it does not move when the phone
    /// crosses a time zone or when daylight saving starts. It does move when the clock
    /// is set.
    pub fn universal() -> Self {
        let mut time: TTime = 0;
        // SAFETY: `TTime::UniversalTime()` is a non-virtual, non-static member of a
        // class whose only storage is one `TInt64` (`__DECLARE_TEST` adds no data), so
        // `&mut time` is a valid `TTime*` for `this` under the AAPCS convention
        // observed in experiment 78. It writes the eight bytes and cannot leave.
        unsafe { TTime_UniversalTime(&mut time) };
        Self(time)
    }

    /// The current **local** time (`TTime::HomeTime`): UTC plus the phone's UTC offset
    /// and daylight-saving rule. What the clock on the screen shows.
    pub fn home() -> Self {
        let mut time: TTime = 0;
        // SAFETY: as `Ttime::universal`.
        unsafe { TTime_HomeTime(&mut time) };
        Self(time)
    }

    /// Wraps a raw `TTime` value.
    pub const fn from_micros_since_year_zero(micros: i64) -> Self {
        Self(micros)
    }

    /// The raw `TTime` value: microseconds since 1 January 0 AD.
    pub const fn micros_since_year_zero(self) -> i64 {
        self.0
    }
}
