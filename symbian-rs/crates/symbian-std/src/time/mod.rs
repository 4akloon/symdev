//! `std::time` for Symbian: [`Duration`], [`Instant`], [`SystemTime`], [`UNIX_EPOCH`]
//! and [`SystemTimeError`], over the clocks `euser` offers (design spec §6a, §11
//! step 76).
//!
//! ```ignore
//! use symbian_std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
//!
//! let started = Instant::now()?;
//! do_the_work();
//! let took: Duration = started.elapsed();
//!
//! let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
//! ```
//!
//! [`Duration`] is not redefined here: it is `core::time::Duration`, the same type
//! `std::time` re-exports, so a duration crosses this boundary unchanged.
//!
//! # Which Symbian clock is which
//!
//! | | Counter | Period | Wraps after | Moves when the clock is set |
//! |---|---|---|---|---|
//! | [`Instant`] | `User::TickCount` | `UserHal::TickPeriod`, **15 625 µs** measured | 2^32 ticks = **776.7 days** | no |
//! | [`SystemTime`] | `TTime::UniversalTime` | **1 µs** measured | ±292 471 years | **yes** |
//!
//! Two counters the SDK deliberately does **not** build on, and why:
//!
//! - **`User::NTickCount`** is finer — 1 000 µs measured in EKA2L1 — but its period is
//!   the HAL attribute `ENanoTickPeriod`, and there is no way to read it here.
//!   `hal.dll` is not on this SDK's link line, and euser's own
//!   `UserSvr::HalGet` answers `KErrNotSupported` for every attribute inside EKA2L1,
//!   attribute 14 included, although `UserHal::TickPeriod` returns that same number.
//!   Using 1 kHz would be an emulator measurement written into the SDK as a device
//!   fact, which this SDK does not do. It is reachable as
//!   `symbian_core::time::NanoTicks` for a program that knows its own board.
//! - **`User::FastCounter`** is finer still — ~33.3 kHz measured — but besides its
//!   frequency, its *direction* is a board property (`HALData::EFastCounterCountsUp`
//!   exists because some boards count down), and that is unreadable for the same
//!   reason. It is `symbian_core::time::FastCounter`.
//!
//! # What cannot be hidden
//!
//! - **The wall clock is the user's, and it can go backwards.** [`SystemTime`] is the
//!   phone's clock: the owner can set it, the network can set it, and a value read a
//!   second ago can be in the future. That is why [`SystemTime::duration_since`]
//!   returns a `Result` — exactly as in `std`, and for exactly the same reason. Use
//!   [`Instant`] to measure how long something took; it is not affected by a clock
//!   change.
//! - **A monotonic reading is coarse.** `Instant`'s resolution is the system tick
//!   period, which measured **15 625 µs** — a sixty-fourth of a second. An `elapsed()`
//!   over anything shorter than that is 0 or one whole tick, and no averaging in this
//!   crate can invent the bits in between. `symbian_core::time::FastCounter` is there
//!   for a program that must do better and knows what its board's counter means.
//! - **The emulator is not a device.** Every period above was measured inside EKA2L1
//!   (experiment 85). EKA2L1's wall clock tracked the host's to within 0.4 s, and its
//!   `UserHal::TickPeriod` reports the same 1/64 s a phone does, but a device has
//!   measured none of this.
//! - **`Instant::now()` returns a `Result`**, where `std`'s does not. The tick period
//!   comes from `UserHal::TickPeriod`, which returns a `TInt`, and nothing in this
//!   crate may panic — so the failure is in the signature instead of hidden behind an
//!   assumption. Every other method keeps `std`'s exact shape.
//! - **[`Instant`] is not `Ord`.** The counter behind it is 32 bits and wraps, and a
//!   wrapping counter has no total order: the RFC 1982 "is it before or after" test is
//!   not transitive, so implementing `Ord` would be a lie that `BTreeMap` would find
//!   first. [`Instant::checked_duration_since`] returning `None` is the honest
//!   comparison, and it is `std`'s own API.
mod instant;
mod system_time;

/// `std::time::Duration` is `core::time::Duration`; so is this one.
pub use core::time::Duration;

pub use instant::Instant;
pub use system_time::{SystemTime, SystemTimeError, UNIX_EPOCH};
