//! Picking a piece's path from its types, on stable Rust.
//!
//! A proc macro sees tokens, not types, so the choice between "append directly" and
//! "format through `core::fmt`" is made by the type checker, with autoref
//! specialisation: the expansion calls `(&&&Probe::of(d, a)).__symbian_kind()`, and
//! method resolution tries the receiver `&&&Probe` first, then `&&Probe`, then `&Probe`.
//! Each level has one trait whose impl applies only under its bounds:
//!
//! | receiver    | trait       | applies when                     | tag        |
//! |-------------|-------------|----------------------------------|------------|
//! | `&&&Probe`  | `SinkKind`  | `D: Sink`, `A: Arg`              | `SinkTag`  |
//! | `&&Probe`   | `WriteKind` | `D: fmt::Write`, `A: Arg`        | `WriteTag` |
//! | `&Probe`    | `SlowKind`  | always                           | `SlowTag`  |
//!
//! The types must be known where the macro expands, which they are in ordinary code;
//! inside a generic function over `W: fmt::Write` only the bound is known and the
//! `fmt::Write` path is taken, which is still exact. The slow tag calls the closure
//! the expansion wrote at the call site, so a type that is not on the list is
//! formatted by `write_fmt(format_args!("{}", a))`, with whichever `Write` trait the
//! caller has in scope, and returns what that returns.

use core::fmt;
use core::marker::PhantomData;

use crate::arg::Arg;
use crate::sink::{Generic, Sink};

/// The types of one piece: the destination `D` and the argument `A`.
pub struct Probe<D: ?Sized, A: ?Sized>(PhantomData<(fn(&D), fn(&A))>);

impl<D: ?Sized, A: ?Sized> Probe<D, A> {
    pub fn of(_: &D, _: &A) -> Self {
        Self(PhantomData)
    }
}

pub trait SinkKind {
    fn __symbian_kind(&self) -> SinkTag {
        SinkTag
    }
}
impl<D: Sink + ?Sized, A: Arg + ?Sized> SinkKind for &&Probe<D, A> {}

pub trait WriteKind {
    fn __symbian_kind(&self) -> WriteTag {
        WriteTag
    }
}
impl<D: fmt::Write + ?Sized, A: Arg + ?Sized> WriteKind for &Probe<D, A> {}

pub trait SlowKind {
    fn __symbian_kind(&self) -> SlowTag {
        SlowTag
    }
}
impl<D: ?Sized, A: ?Sized> SlowKind for Probe<D, A> {}

pub struct SinkTag;
pub struct WriteTag;
pub struct SlowTag;

impl SinkTag {
    #[inline(always)]
    pub fn put<D, A, F>(self, d: &mut D, a: &A, _slow: F) -> fmt::Result
    where
        D: Sink + ?Sized,
        A: Arg + ?Sized,
        F: FnOnce(&mut D, &A) -> fmt::Result,
    {
        a.put(d)
    }
}

impl WriteTag {
    #[inline(always)]
    pub fn put<D, A, F>(self, d: &mut D, a: &A, _slow: F) -> fmt::Result
    where
        D: fmt::Write + ?Sized,
        A: Arg + ?Sized,
        F: FnOnce(&mut D, &A) -> fmt::Result,
    {
        a.put(&mut Generic(d))
    }
}

impl SlowTag {
    #[inline(always)]
    pub fn put<D, A, R, F>(self, d: &mut D, a: &A, slow: F) -> R
    where
        D: ?Sized,
        A: ?Sized,
        F: FnOnce(&mut D, &A) -> R,
    {
        slow(d, a)
    }
}

/// The one method call on the destination: `dst.__symbian_fmt_enter(args, body)`
/// evaluates `dst` once and borrows it as `write_fmt` would, then the arguments.
pub trait Enter {
    #[inline(always)]
    fn __symbian_fmt_enter<T, R>(&mut self, args: T, body: impl FnOnce(&mut Self, T) -> R) -> R {
        body(self, args)
    }
}
impl<W: ?Sized> Enter for W {}
