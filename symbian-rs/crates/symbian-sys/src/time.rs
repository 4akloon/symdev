//! The clocks: `User`'s three counters, `UserHal::TickPeriod` and `TTime`'s two
//! "what time is it" members. All are `euser.dll` exports, and all of the mangled
//! names below are the unversioned part of what `nm -D
//! epoc32/release/armv5/lib/euser.dso` prints (S60 3rd FP2).
//!
//! **None of these needs the C++ shim**, against the three rules in
//! `shims/common/symrs_shim.h`: no declaration in `e32std.h` or `e32hal.h` says any of
//! them can leave and none carries an `L` suffix; every signature is scalar in and
//! scalar or `void` out; and `TTime::HomeTime`/`UniversalTime` are non-virtual,
//! non-static members of a class with no base, which experiment 78 observed to be an
//! ordinary AAPCS call with `this` as argument 0.

/// `TTime` is one `TInt64 iTime` — microseconds since midnight, 1 January 0 AD nominal
/// Gregorian (`e32std.h` line 1770). `__DECLARE_TEST` (`e32def.h` line 2024) adds only
/// member *functions*, so the class has no other storage and `sizeof(TTime) == 8`: to
/// Rust it is an `i64` and nothing else.
pub type TTime = i64;

unsafe extern "C" {
    /// `00000a84 T _ZN4User9TickCountEv` — `User::TickCount()`, the **system** tick
    /// counter (`e32std.h` line 4511). `TUint`, so it wraps at 2^32 ticks.
    #[link_name = "_ZN4User9TickCountEv"]
    pub fn User_TickCount() -> u32;

    /// `000020d8 T _ZN4User10NTickCountEv` — `User::NTickCount()`, the **nanokernel**
    /// tick counter (`e32std.h` line 4513). `TUint32`, so it wraps at 2^32 ticks.
    #[link_name = "_ZN4User10NTickCountEv"]
    pub fn User_NTickCount() -> u32;

    /// `0000091c T _ZN4User11FastCounterEv` — `User::FastCounter()` (`e32std.h` line
    /// 4517). Its frequency is `HALData::EFastCounterFrequency` and it **may count
    /// down**: `HALData::EFastCounterCountsUp` exists precisely because the direction
    /// is a property of the board (`hal_data.h`). Both attributes live in `hal.dll`,
    /// which is not on this SDK's link line, so neither is read here.
    #[link_name = "_ZN4User11FastCounterEv"]
    pub fn User_FastCounter() -> u32;

    /// `000012ac T _ZN7UserHal10TickPeriodER27TTimeIntervalMicroSeconds32` —
    /// `UserHal::TickPeriod(TTimeIntervalMicroSeconds32&)` (`e32hal.h` line 589): the
    /// period of [`User_TickCount`] in microseconds, written through the reference.
    /// Returns a system-wide error code. The reference is to a 4-byte class holding
    /// one `TInt`, so it is an `i32` out-pointer.
    #[link_name = "_ZN7UserHal10TickPeriodER27TTimeIntervalMicroSeconds32"]
    pub fn UserHal_TickPeriod(period: *mut i32) -> i32;

    /// `0000208c T _ZN4User10SetUTCTimeERK5TTime` —
    /// `User::SetUTCTime(const TTime&)`: sets the device's UTC clock, returning a
    /// system-wide error code. Needs the `WriteDeviceData` capability.
    #[link_name = "_ZN4User10SetUTCTimeERK5TTime"]
    pub fn User_SetUTCTime(time: *const TTime) -> i32;

    /// `00000d68 T _ZN5TTime8HomeTimeEv` — `TTime::HomeTime()`: sets `*this` to the
    /// current **local** time, the one the phone's clock and time-zone setting show.
    #[link_name = "_ZN5TTime8HomeTimeEv"]
    pub fn TTime_HomeTime(this: *mut TTime);

    /// `00000d58 T _ZN5TTime13UniversalTimeEv` — `TTime::UniversalTime()`: sets `*this`
    /// to the current **UTC** time.
    #[link_name = "_ZN5TTime13UniversalTimeEv"]
    pub fn TTime_UniversalTime(this: *mut TTime);
}
