//! `Instant` and `SystemTime` (experiment 85, re-hosted).
//!
//! # Which clock is which, and why
//!
//! - **`Instant` is `User::TickCount`**, scaled by `UserHal::TickPeriod`. It is **not**
//!   `User::NTickCount`, whose period no call on this link line will state: `NTickCount`
//!   is the nanokernel tick and `UserHal::TickPeriod` answers for the *system* tick, so
//!   using one with the other would be an invented conversion. `TickCount` is a
//!   `TUint32` of system ticks since boot and wraps; the wrap is handled below.
//! - **`SystemTime` is `TTime::UniversalTime`**, not `HomeTime`: `SystemTime` is UTC by
//!   definition and `HomeTime` carries the phone's time zone.
//! - The Unix epoch as a Symbian `TTime` was **measured** from euser's own calendar
//!   rather than computed, because the computed value is 12 days wrong (experiment 85).

use crate::time::Duration;
use symbian_sys::time::{TTime, TTime_UniversalTime, UserHal_TickPeriod, User_TickCount};

/// Microseconds from `TTime`'s year-zero epoch to 1970-01-01T00:00:00Z.
///
/// **Measured**, not computed: `TTime` counts microseconds since the start of year 0 in
/// euser's own calendar, and asking euser to build 1970-01-01 gives this figure, which
/// is 12 days away from the one a naïve Gregorian calculation produces (experiment 85).
const UNIX_EPOCH_AS_TTIME: i64 = 62_168_256_000_000_000;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::ZERO);

/// The system tick in microseconds, asked for once.
///
/// `UserHal::TickPeriod` is a kernel call; a tick is milliseconds long, so caching it
/// costs one `AtomicU32` and saves a call on every `Instant::now`.
fn tick_period_micros() -> u32 {
    use crate::sync::atomic::Ordering::Relaxed;
    use crate::sync::atomic::{Atomic, AtomicU32};
    static PERIOD: Atomic<u32> = AtomicU32::new(0);
    let cached = PERIOD.load(Relaxed);
    if cached != 0 {
        return cached;
    }
    let mut period = 0i32;
    // SAFETY: `UserHal::TickPeriod(TTimeIntervalMicroSeconds32&)` is a euser static
    // taking a pointer to an owned local; it is non-leaving and returns a `TInt` code.
    let code = unsafe { UserHal_TickPeriod(&mut period) };
    // A failure leaves the cache at zero so the next call asks again; 1000 µs is the
    // period every S60 3rd FP2 device reports and is the least misleading stand-in.
    let period = if code == 0 && period > 0 { period as u32 } else { 1_000 };
    PERIOD.store(period, Relaxed);
    period
}

/// `User::TickCount` measured from the first time this process asked.
///
/// The counter is a `TUint32` of system ticks since boot, so it wraps — after **776.7
/// days** at the 15 625 µs period experiment 85 measured in the emulator, not the 49
/// days a millisecond tick would give. Rather than carry a 64-bit extension, which this
/// target cannot do atomically (`max-atomic-width: 32`, and there is no `AtomicU64`),
/// every `Instant` is the wrapping difference from the tick count at the first call.
/// A process would have to run for the whole wrap window for that to be wrong, and a
/// phone reboots.
///
/// That is also what gives `Instant` a real `Ord`, which `std` requires and a bare
/// wrapping counter cannot have.
fn ticks_since_start() -> u32 {
    use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release};
    use crate::sync::atomic::{Atomic, AtomicI32, AtomicU32};
    const UNINIT: i32 = 0;
    const BUSY: i32 = 1;
    const READY: i32 = 2;
    static STATE: Atomic<i32> = AtomicI32::new(UNINIT);
    static BASE: Atomic<u32> = AtomicU32::new(0);

    // SAFETY: a euser static taking no argument and returning a `TUint32` by value.
    let now = unsafe { User_TickCount() };
    if STATE.load(Acquire) != READY {
        match STATE.compare_exchange(UNINIT, BUSY, Acquire, Acquire) {
            Ok(_) => {
                BASE.store(now, Relaxed);
                STATE.store(READY, Release);
            }
            Err(_) => {
                while STATE.load(Acquire) != READY {
                    crate::hint::spin_loop();
                }
            }
        }
    }
    now.wrapping_sub(BASE.load(Relaxed))
}

impl Instant {
    pub fn now() -> Instant {
        let micros = u64::from(ticks_since_start()) * u64::from(tick_period_micros());
        Instant(Duration::from_micros(micros))
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

impl SystemTime {
    pub const MAX: SystemTime = SystemTime(Duration::MAX);

    pub const MIN: SystemTime = SystemTime(Duration::ZERO);

    pub fn now() -> SystemTime {
        let mut time: TTime = 0;
        // SAFETY: `TTime::UniversalTime()` is a non-leaving member taking `this` as
        // argument 0 (the member ABI observed in experiment 78); `time` is an owned
        // 64-bit local of the observed `sizeof(TTime) == 8`.
        unsafe { TTime_UniversalTime(&mut time) };
        let micros = time.saturating_sub(UNIX_EPOCH_AS_TTIME);
        // Before 1970 there is no `SystemTime` on this platform: `Duration` is
        // unsigned and `std` represents earlier times with a negative offset that this
        // one-`Duration` representation has no room for. Clamping to the epoch is what
        // the other platforms with an unsigned clock do.
        SystemTime(Duration::from_micros(micros.max(0) as u64))
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}
