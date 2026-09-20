//! [`SystemTime`], [`UNIX_EPOCH`] and [`SystemTimeError`]: `std`'s wall clock over
//! `TTime::UniversalTime`.
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;

use symbian_core::time::Ttime;

/// Microseconds in a second.
const MICROS_PER_SEC: u64 = 1_000_000;

/// Microseconds from the `TTime` origin — midnight, 1 January 0 AD — to
/// 1970-01-01T00:00:00Z.
///
/// **Measured, not computed** (experiment 85). Symbian's calendar is Julian before
/// 1600 and Gregorian from 1600 on, which is what `e32std.h`'s "nominal Gregorian"
/// means, so proleptic Gregorian arithmetic gives 719 528 days and is **12 days
/// wrong**. euser's own `Time::LeapYearsUpTo(1970)` answers **490**, for
/// 1970·365 + 490 = 719 540 days, and euser's own `TTime` accessors decode this value
/// as day 1 of the year, day 0 of the month, weekday 3 = `EThursday`, in a 31-day
/// month: 1 January 1970, a Thursday, which is the date it has to be.
const UNIX_EPOCH_MICROS: i64 = 62_168_256_000_000_000;

/// An anchor in time from which [`SystemTime`] durations are measured, as
/// `std::time::UNIX_EPOCH`.
pub const UNIX_EPOCH: SystemTime = SystemTime(UNIX_EPOCH_MICROS);

/// A measurement of the system clock, as `std::time::SystemTime`.
///
/// This is the phone's clock and it is **not monotonic**: the owner can set it, the
/// network can set it, and a value read a moment ago may now be in the future. Use it
/// for a timestamp, never for how long something took — that is
/// [`Instant`](super::Instant).
///
/// It is backed by `TTime::UniversalTime`, which is **UTC**, because `UNIX_EPOCH` is a
/// UTC instant and so every `SystemTime` must be. `TTime::HomeTime`, the local time
/// the phone displays, is `symbian_core::time::Ttime::home`; inside EKA2L1 the two
/// differed by exactly the emulated time zone.
///
/// Stored as microseconds since the `TTime` origin, so the resolution is `TTime`'s
/// own: 1 µs, measured.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime(i64);

impl SystemTime {
    /// `std::time::SystemTime::UNIX_EPOCH`.
    pub const UNIX_EPOCH: SystemTime = UNIX_EPOCH;

    /// Reads the system clock (`TTime::UniversalTime`), as `std::time::SystemTime::now`.
    ///
    /// Infallible, and `std`-shaped: `TTime::UniversalTime` returns nothing but the
    /// time.
    pub fn now() -> Self {
        Self(Ttime::universal().micros_since_year_zero())
    }

    /// The amount of time from `earlier` to this one, as
    /// `std::time::SystemTime::duration_since`.
    ///
    /// The `Err` says `earlier` is in fact the later of the two, which on a clock the
    /// user can set is an ordinary thing to happen rather than a bug; it carries how
    /// far the wrong way round they are.
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        match self.0.checked_sub(earlier.0) {
            Some(micros) if micros >= 0 => Ok(duration_of(micros as u64)),
            Some(micros) => Err(SystemTimeError(duration_of(micros.unsigned_abs()))),
            // Only reachable between the two extremes of the representable range.
            None => Err(SystemTimeError(Duration::MAX)),
        }
    }

    /// How much time has passed since this reading, as `std::time::SystemTime::elapsed`.
    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(*self)
    }

    /// `duration` later, or `None` if that leaves the representable range, as
    /// `std::time::SystemTime::checked_add`.
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        self.0.checked_add(micros_of(duration)?).map(Self)
    }

    /// `duration` earlier, or `None` if that leaves the representable range, as
    /// `std::time::SystemTime::checked_sub`.
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        self.0.checked_sub(micros_of(duration)?).map(Self)
    }

    /// The raw `TTime`: microseconds since midnight, 1 January 0 AD nominal Gregorian.
    ///
    /// The escape hatch for a program that has to hand a `TTime` to a Symbian API this
    /// SDK does not cover.
    pub const fn as_ttime(&self) -> Ttime {
        Ttime::from_micros_since_year_zero(self.0)
    }

    /// A `SystemTime` from a raw `TTime`.
    pub const fn from_ttime(time: Ttime) -> Self {
        Self(time.micros_since_year_zero())
    }
}

/// Returned by [`SystemTime::duration_since`] when the earlier time is the later one,
/// as `std::time::SystemTimeError`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
    /// How far the wrong way round the two times were, as
    /// `std::time::SystemTimeError::duration`.
    pub const fn duration(&self) -> Duration {
        self.0
    }
}

impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "second time provided was later than self")
    }
}

impl core::error::Error for SystemTimeError {}

/// Microseconds since 1 January 0 AD, with the Unix seconds alongside when the value
/// is on that side of the epoch — the two numbers anyone debugging this actually wants.
impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SystemTime {{ ttime_micros: {}", self.0)?;
        if let Ok(since_epoch) = self.duration_since(UNIX_EPOCH) {
            write!(f, ", unix_secs: {}", since_epoch.as_secs())?;
        }
        f.write_str(" }")
    }
}

/// `std` panics when the sum leaves the representable range; this crate does not
/// panic, so it saturates at the end of the range. [`SystemTime::checked_add`] is the
/// form that reports it.
impl Add<Duration> for SystemTime {
    type Output = SystemTime;

    fn add(self, duration: Duration) -> SystemTime {
        self.checked_add(duration).unwrap_or(SystemTime(i64::MAX))
    }
}

impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

/// As [`Add`]: saturating, with [`SystemTime::checked_sub`] as the reporting form.
impl Sub<Duration> for SystemTime {
    type Output = SystemTime;

    fn sub(self, duration: Duration) -> SystemTime {
        self.checked_sub(duration).unwrap_or(SystemTime(i64::MIN))
    }
}

impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}

/// A non-negative microsecond count as a [`Duration`].
const fn duration_of(micros: u64) -> Duration {
    Duration::new(
        micros / MICROS_PER_SEC,
        (micros % MICROS_PER_SEC) as u32 * 1_000,
    )
}

/// A [`Duration`] as signed microseconds, or `None` if it does not fit.
///
/// `u64` and not `Duration::as_micros`'s `u128`, to keep 128-bit division out of an
/// image that would otherwise never need it.
const fn micros_of(duration: Duration) -> Option<i64> {
    let Some(whole) = duration.as_secs().checked_mul(MICROS_PER_SEC) else {
        return None;
    };
    let Some(micros) = whole.checked_add(duration.subsec_micros() as u64) else {
        return None;
    };
    if micros > i64::MAX as u64 {
        return None;
    }
    Some(micros as i64)
}
