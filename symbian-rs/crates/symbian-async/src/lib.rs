//! `async`/`await` on Symbian's active objects (design spec §8, §11 step 73).
//!
//! Every asynchronous call on this platform takes a `TRequestStatus&` and completes it
//! later, and there are exactly two ways to wait for one. This crate is the second.
//!
//! ```ignore
//! use symbian_async::{block_on, join, sleep};
//! use core::time::Duration;
//!
//! block_on(async {
//!     let (a, b) = join(
//!         sleep(Duration::from_millis(300)),
//!         sleep(Duration::from_millis(100)),
//!     )
//!     .await;
//!     a?;
//!     b
//! })?
//! ```
//!
//! Both sleeps are outstanding at once and the whole thing takes about 300 ms, not 400.
//!
//! # The two models, and why they must not be mixed
//!
//! **Blocking** — `symbian_core::net::blocking`, and everything in `symbian_std::net`
//! and `symbian_std::fs` built on it — issues one request and calls
//! `User::WaitForRequest`. That call waits on the **thread's** request semaphore, which
//! every request of every kind on that thread signals, and then checks whether *this*
//! status has been completed; if it has not, it waits again. It is correct precisely
//! because the status is private to the call and nothing else on the thread has a
//! request outstanding.
//!
//! **This crate** has many requests outstanding by design. Their statuses live in
//! `CActive` objects on a `CActiveScheduler`, and the scheduler is the thing that reads
//! the semaphore, works out which object's status was completed, and calls its `RunL`.
//! That is what an active scheduler is *for*.
//!
//! Run both on one thread and the accounting breaks in both directions: a
//! `User::WaitForRequest` consumes a semaphore signal that belonged to an active
//! object, so the scheduler's next `WaitForAnyRequest` blocks although a request has
//! completed; and the scheduler consumes the signal the blocking call was waiting for,
//! so the blocking call waits for ever. Neither failure names itself — the program just
//! stops.
//!
//! ## What stops it
//!
//! `symbian_core::net::blocking` **refuses to run on a thread that has a
//! `CActiveScheduler` installed**, with `KErrInUse`. That is not a flag this crate
//! sets: it is the platform's own rule, read off the one call that answers it
//! (`CActiveScheduler::Current()`), and it holds whoever installed the scheduler —
//! [`block_on`], or CONE in an Avkon application, where blocking the UI thread was
//! always a bug.
//!
//! So the two models cannot meet by accident:
//!
//! - a blocking program has no scheduler, and `sleep` reports `KErrNotReady` if it is
//!   awaited there;
//! - inside [`block_on`] or an Avkon application, a blocking socket or file call
//!   reports `KErrInUse` instead of eating a completion;
//! - and the two never share a `TRequestStatus`, because neither hands one out:
//!   `blocking` keeps its on its own stack and [`Request`] keeps its inside the
//!   `CActive` that owns it.
//!
//! The way to have both is two threads: a worker with no scheduler may block all it
//! likes, and `symbian_std::thread` is how it is made. Note the one hole this SDK
//! cannot close yet: `JoinHandle::join` is itself a `User::WaitForRequest`, so joining
//! a thread from inside an async task has the same hazard and nothing refuses it.
//!
//! # Shape
//!
//! - [`block_on`] owns a scheduler: a console application's `fn main` runs its whole
//!   asynchronous life inside one call.
//! - [`spawn`] joins the scheduler that is already there, which is the only thing a GUI
//!   application may do — CONE owns the event loop and `CCoeEnv` is a `CActive` on it
//!   (`docs/research/avkon-rust-spec.md` §1.3).
//! - One executor per process and per thread, checked: it binds to the scheduler of
//!   the thread that first uses it and refuses another.
//! - [`sleep`] is the asynchronous source this step ships. [`Request`] and [`Source`]
//!   are what a second one needs — `RSocket`'s `Connect`, `Send`, `RecvOneOrMore` and
//!   `Accept` are already `TRequestStatus`-shaped, and `RSocket::CancelAll` is their
//!   `cancel`, so they need no new C++ and nothing new here.
//!
//! # Cost
//!
//! One `CActive` (28 bytes) and one heap block per outstanding request, one `Rc` task
//! per [`spawn`], two boxes per [`join`]. A wake is one atomic store and a poll round
//! one atomic swap per task; on this device an atomic is a kernel call over a
//! process-wide lock, about 90 times a plain increment (experiment 72), which is why
//! the executor scans a short list of flags rather than sharing a queue.
#![no_std]

extern crate alloc;

mod combinator;
mod executor;
mod request;
mod run;
mod scheduler;
mod task;
mod timer;
mod waker;

pub use combinator::{Either, Join, Race, join, race};
pub use request::{Request, Source};
pub use run::{block_on, spawn};
pub use timer::{MAX_SLEEP_MICROS, Sleep, sleep};
