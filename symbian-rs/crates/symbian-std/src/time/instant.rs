//! [`Instant`]: `std`'s monotonic clock over `User::TickCount`.
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;

use symbian_core::time::SystemTicks;

use crate::io::Result;

/// A measurement of a monotonically non-decreasing clock, as `std::time::Instant`.
///
/// Opaque and useful only with [`Duration`]. It is **not** affected by the device
/// clock being set, which is the whole reason it exists next to
/// [`SystemTime`](super::SystemTime).
///
/// ```ignore
/// let started = Instant::now()?;
/// symbian_core::user::after(500_000);
/// assert!(started.elapsed() >= Duration::from_millis(480));
/// ```
///
/// # Divergences from `std`, and why
///
/// - [`Instant::now`] returns a [`Result`]: the tick period comes from
///   `UserHal::TickPeriod`, which returns a `TInt`, and this crate does not panic.
/// - There is **no `Ord`**. `User::TickCount` is a `TUint` that wraps at 2^32 ticks —
///   776.7 days at the 15 625 µs period measured in EKA2L1 — and a wrapping counter
///   has no total order. Use [`Instant::checked_duration_since`], which is `std`'s.
/// - Where `std` would panic on overflow, this saturates or wraps and says so on the
///   method. The `checked_*` pair is how a program detects it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Instant {
    ticks: SystemTicks,
    /// The period of `ticks`, in microseconds, as the platform reported it when this
    /// instant was taken. Carried rather than re-read so that every method below can
    /// keep `std`'s infallible signature.
    period_micros: u32,
}

impl Instant {
    /// Reads the monotonic clock (`User::TickCount`), together with the period
    /// (`UserHal::TickPeriod`) that turns its ticks into a [`Duration`].
    ///
    /// The `Err` is the one `UserHal::TickPeriod` returned; a period of zero or less
    /// is `KErrNotSupported`, because a clock that claims a zero-length tick cannot
    /// measure anything and quietly dividing by it would be worse.
    pub fn now() -> Result<Self> {
        let period = SystemTicks::period_micros()?;
        if period <= 0 {
            return Err(crate::io::Error::from_raw_os_error(NOT_SUPPORTED));
        }
        Ok(Self {
            ticks: SystemTicks::now(),
            period_micros: period as u32,
        })
    }

    /// The time elapsed from `earlier` to this instant, or zero if `earlier` is
    /// actually the later of the two — `std::time::Instant::duration_since`, which has
    /// saturated rather than panicked since Rust 1.60.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.saturating_duration_since(earlier)
    }

    /// The time elapsed from `earlier` to this instant, or `None` if `earlier` is
    /// later, as `std::time::Instant::checked_duration_since`.
    ///
    /// "Later" on a wrapping counter is a local judgement: the difference is taken
    /// modulo 2^32 and a result in the upper half of the range is read as `earlier`
    /// being in the future. Two instants more than half the wrap window apart —
    /// 388 days at the measured tick period — cannot be told apart from that, and
    /// there is no bit left anywhere to do it with.
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        let ticks = self.ticks.ticks_since(earlier.ticks);
        if ticks > u32::MAX / 2 {
            return None;
        }
        Some(self.duration_of(ticks))
    }

    /// The time elapsed from `earlier` to this instant, or [`Duration::ZERO`] if
    /// `earlier` is later, as `std::time::Instant::saturating_duration_since`.
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier)
            .unwrap_or(Duration::ZERO)
    }

    /// How long since this instant was taken, as `std::time::Instant::elapsed`.
    ///
    /// Zero if the clock could not be read at all, which is the only thing left when
    /// `std`'s signature returns a bare [`Duration`]; [`Instant::now`] is the place
    /// that reports such a failure.
    pub fn elapsed(&self) -> Duration {
        match Instant::now() {
            Ok(now) => now.saturating_duration_since(*self),
            Err(_) => Duration::ZERO,
        }
    }

    /// The instant `duration` later, or `None` if that is far enough ahead to be
    /// unrepresentable — as `std::time::Instant::checked_add`.
    ///
    /// The limit here is not arithmetic overflow but the counter: past half a wrap
    /// window the result could not be told from a time in the past.
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        let ticks = self.ticks_of(duration)?;
        Some(Self {
            ticks: self.ticks.advanced_by(ticks),
            period_micros: self.period_micros,
        })
    }

    /// The instant `duration` earlier, or `None` if that is unrepresentable — as
    /// `std::time::Instant::checked_sub`. Same limit as [`Instant::checked_add`].
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        let ticks = self.ticks_of(duration)?;
        Some(Self {
            ticks: self.ticks.advanced_by(ticks.wrapping_neg()),
            period_micros: self.period_micros,
        })
    }

    /// The raw `User::TickCount` value this instant holds, for a program that has gone
    /// past what this module offers.
    pub const fn raw_ticks(&self) -> u32 {
        self.ticks.raw()
    }

    /// The tick period, in microseconds, that `UserHal::TickPeriod` reported when this
    /// instant was taken — the resolution of every duration derived from it.
    pub const fn tick_period_micros(&self) -> u32 {
        self.period_micros
    }

    /// `ticks` as a duration. `u64` throughout: 2^32 ticks of 15 625 µs is 6.7e13 µs,
    /// which no `u32` holds and every `u64` does.
    fn duration_of(&self, ticks: u32) -> Duration {
        Duration::from_micros(ticks as u64 * self.period_micros as u64)
    }

    /// `duration` as a whole number of ticks, truncated towards zero, or `None` if it
    /// reaches half the counter's range.
    ///
    /// `u64` and not `Duration::as_micros`'s `u128`: 128-bit division is a
    /// `compiler_builtins` routine this platform would have to carry, and a duration
    /// that does not fit in `u64` microseconds — 584 542 years — is out of range here
    /// anyway.
    fn ticks_of(&self, duration: Duration) -> Option<u32> {
        let micros = duration
            .as_secs()
            .checked_mul(MICROS_PER_SEC)?
            .checked_add(duration.subsec_micros() as u64)?;
        let ticks = micros / self.period_micros as u64;
        if ticks > (u32::MAX / 2) as u64 {
            return None;
        }
        Some(ticks as u32)
    }
}

/// `KErrNotSupported`, the code for a tick period that cannot measure anything.
const NOT_SUPPORTED: i32 = -5;

/// Microseconds in a second.
const MICROS_PER_SEC: u64 = 1_000_000;

/// Ticks rather than a duration, because a duration would need the period and this is
/// the one place that must not fail.
impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Instant {{ ticks: {}, period_micros: {} }}",
            self.ticks.raw(),
            self.period_micros
        )
    }
}

/// `std` panics when the sum is unrepresentable; this wraps, because the counter it is
/// made of wraps and nothing here may panic. [`Instant::checked_add`] is the guarded
/// form.
impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        let ticks = self.ticks_of(duration).unwrap_or(0);
        Instant {
            ticks: self.ticks.advanced_by(ticks),
            period_micros: self.period_micros,
        }
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

/// As [`Add`]: wrapping, with [`Instant::checked_sub`] as the guarded form.
impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, duration: Duration) -> Instant {
        let ticks = self.ticks_of(duration).unwrap_or(0);
        Instant {
            ticks: self.ticks.advanced_by(ticks.wrapping_neg()),
            period_micros: self.period_micros,
        }
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}

/// `std::time::Instant - Instant`, which is [`Instant::duration_since`].
impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }
}
