//! The two kernel synchronisation objects `std`'s `Mutex` and `Condvar` are built on,
//! from `nm -D epoc32/release/armv5/lib/euser.dso`.
//!
//! [`crate::thread`] already declares `RFastLock` and `RSemaphore`, which is what
//! `symbian_std::sync::Mutex` uses. A real `std::sync::Condvar` needs more than a
//! semaphore: `Condvar::wait` has to release a mutex and reacquire it atomically with
//! respect to the notification, and the only object on this platform that does that is
//! `RCondVar`, whose `Wait` takes the `RMutex` it is paired with.
//!
//! Both are declared in `e32std.h` (`RMutex` at line 3325, `RCondVar` at 3354) and both
//! are exported by this ROM's euser, which is why they may be named here at all.
//!
//! # What is *not* stated anywhere
//!
//! `e32std.h` says nothing about whether `RMutex::Wait` is recursive — whether a thread
//! that already holds the mutex may take it again. `std::sync::Mutex` requires that it
//! may **not**, so the SDK does not use `RMutex` to build one: it stays a semaphore of
//! one token (experiment 80), and `RMutex` appears only as `RCondVar`'s partner, where
//! the lock and unlock are always balanced and the question does not arise.

/// `RMutex`: a kernel mutex handle. One `TInt`, like every `RHandleBase`.
#[repr(C)]
pub struct RMutex {
    pub handle: i32,
}

impl RMutex {
    /// An unopened mutex, as a value: see [`crate::thread::RSemaphore::NULL`].
    pub const NULL: Self = Self { handle: 0 };

    pub const fn null() -> Self {
        Self::NULL
    }
}

/// `RCondVar`: a kernel condition variable handle. One `TInt`.
#[repr(C)]
pub struct RCondVar {
    pub handle: i32,
}

impl RCondVar {
    /// An unopened condition variable, as a value: see
    /// [`crate::thread::RSemaphore::NULL`].
    pub const NULL: Self = Self { handle: 0 };

    pub const fn null() -> Self {
        Self::NULL
    }
}

unsafe extern "C" {
    /// `00000e54 T _ZN6RMutex11CreateLocalE10TOwnerType` —
    /// `RMutex::CreateLocal(TOwnerType)`. Returns a system error code.
    #[link_name = "_ZN6RMutex11CreateLocalE10TOwnerType"]
    pub fn RMutex_CreateLocal(this: *mut RMutex, owner: i32) -> i32;

    /// `00000e64 T _ZN6RMutex4WaitEv` — `RMutex::Wait()`.
    #[link_name = "_ZN6RMutex4WaitEv"]
    pub fn RMutex_Wait(this: *mut RMutex);

    /// `00000e68 T _ZN6RMutex6SignalEv` — `RMutex::Signal()`.
    #[link_name = "_ZN6RMutex6SignalEv"]
    pub fn RMutex_Signal(this: *mut RMutex);

    /// `00002080 T _ZN6RMutex6IsHeldEv` — `RMutex::IsHeld()`: whether the *calling*
    /// thread holds it.
    #[link_name = "_ZN6RMutex6IsHeldEv"]
    pub fn RMutex_IsHeld(this: *mut RMutex) -> i32;

    /// `0000143c T _ZN8RCondVar11CreateLocalE10TOwnerType` —
    /// `RCondVar::CreateLocal(TOwnerType)`. Returns a system error code.
    #[link_name = "_ZN8RCondVar11CreateLocalE10TOwnerType"]
    pub fn RCondVar_CreateLocal(this: *mut RCondVar, owner: i32) -> i32;

    /// `0000144c T _ZN8RCondVar4WaitER6RMutex` — `RCondVar::Wait(RMutex&)`: releases
    /// the mutex, blocks, and reacquires it before returning. Returns a system error
    /// code.
    #[link_name = "_ZN8RCondVar4WaitER6RMutex"]
    pub fn RCondVar_Wait(this: *mut RCondVar, mutex: *mut RMutex) -> i32;

    /// `00001458 T _ZN8RCondVar9TimedWaitER6RMutexi` —
    /// `RCondVar::TimedWait(RMutex&, TInt aTimeout)`, the timeout in microseconds.
    /// Returns `KErrTimedOut` (-33) when it expires.
    #[link_name = "_ZN8RCondVar9TimedWaitER6RMutexi"]
    pub fn RCondVar_TimedWait(this: *mut RCondVar, mutex: *mut RMutex, timeout_micros: i32) -> i32;

    /// `00001450 T _ZN8RCondVar6SignalEv` — `RCondVar::Signal()`: releases one waiter.
    #[link_name = "_ZN8RCondVar6SignalEv"]
    pub fn RCondVar_Signal(this: *mut RCondVar);

    /// `00001454 T _ZN8RCondVar9BroadcastEv` — `RCondVar::Broadcast()`: releases all.
    #[link_name = "_ZN8RCondVar9BroadcastEv"]
    pub fn RCondVar_Broadcast(this: *mut RCondVar);

    /// `000008b0 T _ZN4Math6RandomEv` — `Math::Random()`, a `TUint32`.
    ///
    /// **What it is seeded from is not observed** (design spec §10 item 4), so this is
    /// the declaration and not a promise: `std`'s use of it is documented where it is
    /// called, in `sys/random/symbian.rs`.
    #[link_name = "_ZN4Math6RandomEv"]
    pub fn Math_Random() -> u32;
}
