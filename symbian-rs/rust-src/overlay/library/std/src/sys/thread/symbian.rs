//! Threads over `RThread` (experiment 80, re-hosted).
//!
//! # One heap, and the access violation that decided how
//!
//! `RThread::Create` has an overload that hands the new thread the creator's
//! allocator. It looks like the right one and it is not: the worker's exit destroys the
//! creator's heap with it, and the process dies with an access violation the moment the
//! first thread ends (experiment 80 reproduced this in C++ as well as in Rust).
//!
//! What works is the **own-heap** overload plus `User::SwitchAllocator` as the worker's
//! very first instruction. The worker is created with a heap of its own, abandons it
//! before it can allocate anything, and runs on the creator's heap from then on — which
//! is what lets a `Box` or an `Arc` cross threads as Rust requires. The moment that
//! becomes possible, the heap needs a lock, so [`Thread::new`] turns
//! [`crate::sys::alloc::serialise_across_threads`] on *before* it creates the thread,
//! while the process still has one.
//!
//! # Thread-local destructors
//!
//! Nothing in the kernel runs one. [`drop_thread_locals`] is called at the end of every
//! thread this module creates, and `rt::symbian_start` calls it for the main thread
//! after `main` returns (experiment 88).

use crate::ffi::CStr;
use crate::io;
use crate::num::NonZero;
use crate::thread::ThreadInit;
use crate::time::Duration;
use symbian_sys::des::Lit16;
use symbian_sys::euser::{RHandleBase, RHandleBase_Close, User_After};
use symbian_sys::thread::{
    EOWNER_PROCESS, KDEFAULT_STACK_SIZE, RThread, RThread_CreateWithOwnHeap, RThread_Logon,
    RThread_Resume, TRequestStatus, User_Allocator, User_SwitchAllocator, User_WaitForRequest,
};

/// What `std::thread::Builder` gets when it asks for a default stack.
pub const DEFAULT_MIN_STACK_SIZE: usize = KDEFAULT_STACK_SIZE as usize;

/// The heap the worker is created with and abandons at once: as small as the kernel
/// will accept, because nothing is ever allocated from it.
const OWN_HEAP_MIN: i32 = 0x1000;
const OWN_HEAP_MAX: i32 = 0x1000;

/// Every worker is created under this name.
///
/// `RThread::Create` refuses a duplicate *global* name, but a thread created with
/// `EOwnerProcess` and a plain name is process-local, so one name for all of them is
/// what the emulator accepted for 23 passing cases in experiment 80. `std`'s own thread
/// name is a Rust-side string and is not this.
static THREAD_NAME: Lit16<10> = Lit16::ascii(b"symbian-rs");

pub struct Thread {
    handle: RThread,
    status: Box<TRequestStatus>,
}

// SAFETY: the kernel handle is `EOwnerProcess`, so any thread of the process may use
// it; the `TRequestStatus` is owned by this value and read only after the worker has
// signalled it.
unsafe impl Send for Thread {}
// SAFETY: as above.
unsafe impl Sync for Thread {}

/// The `TThreadFunction` every spawned thread starts in.
///
/// # Safety
/// `arg` is the `Box<ThreadInit>` [`Thread::new`] leaked, and this is the only thread
/// that will ever see it.
unsafe extern "C" fn trampoline(arg: *mut core::ffi::c_void) -> i32 {
    // SAFETY: switching the heap is the FIRST thing, before anything can allocate or
    // free, so that every allocation this thread makes — including dropping the boxed
    // closure below — goes to the creator's heap and not to the one this thread is
    // about to abandon. `User::Allocator` and `User::SwitchAllocator` are euser statics.
    unsafe { User_SwitchAllocator(CREATOR_HEAP.load()) };
    // SAFETY: the pointer is the `Box<ThreadInit>` the creator leaked for this thread.
    let init = unsafe { Box::from_raw(arg.cast::<ThreadInit>()) };
    init.init()();
    // The thread is ending, so this is where `Drop` runs for whatever it put in a
    // `thread_local!`, on the heap the values were allocated from.
    drop_thread_locals();
    0
}

/// The creator's allocator, published before the first thread exists.
mod creator_heap {
    use crate::cell::UnsafeCell;
    use symbian_sys::thread::RAllocator;

    pub(super) struct CreatorHeap(UnsafeCell<*mut RAllocator>);

    // SAFETY: written once, on the process's only thread, before the thread that reads
    // it exists — the kernel's own thread creation is the happens-before edge.
    unsafe impl Sync for CreatorHeap {}

    impl CreatorHeap {
        pub(super) const fn new() -> Self {
            Self(UnsafeCell::new(crate::ptr::null_mut()))
        }

        /// # Safety
        /// The process must still have exactly one thread.
        pub(super) unsafe fn publish(&self, heap: *mut RAllocator) {
            // SAFETY: single-threaded by the caller's contract.
            unsafe { crate::ptr::write_volatile(self.0.get(), heap) };
        }

        pub(super) fn load(&self) -> *mut RAllocator {
            // SAFETY: a plain aligned pointer load; volatile so it is not cached
            // across the write above.
            unsafe { crate::ptr::read_volatile(self.0.get()) }
        }
    }
}

use creator_heap::CreatorHeap;

static CREATOR_HEAP: CreatorHeap = CreatorHeap::new();

impl Thread {
    /// # Safety
    /// See `thread::Builder::spawn_unchecked`.
    pub unsafe fn new(_stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        // From here on every allocation takes a lock, because two threads will be
        // sharing one heap.
        // SAFETY: a thread cannot have been created without coming through here, and
        // the first caller is therefore on the process's only thread. A later caller
        // finds the flag already set and returns without writing anything.
        let code = unsafe { crate::sys::alloc::serialise_across_threads() };
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code));
        }
        // SAFETY: `User::Allocator` is a euser static returning this thread's heap by
        // reference; it is published before the worker exists and only read by it.
        unsafe { CREATOR_HEAP.publish(User_Allocator()) };

        let mut thread = Thread { handle: RThread::null(), status: Box::new(TRequestStatus::new()) };
        let arg = Box::into_raw(init).cast::<core::ffi::c_void>();
        // SAFETY: the name is a `'static` literal descriptor; `trampoline` matches
        // `TThreadFunction`; `arg` is a leaked box the new thread takes ownership of.
        // The own-heap overload is deliberate — see the module documentation.
        let code = unsafe {
            RThread_CreateWithOwnHeap(
                &mut thread.handle,
                THREAD_NAME.as_desc(),
                trampoline,
                KDEFAULT_STACK_SIZE,
                OWN_HEAP_MIN,
                OWN_HEAP_MAX,
                arg,
                EOWNER_PROCESS,
            )
        };
        if code != 0 {
            // SAFETY: the kernel did not take the argument, so this is still the only
            // reference to the box `spawn` made.
            drop(unsafe { Box::from_raw(arg.cast::<ThreadInit>()) });
            return Err(io::Error::from_raw_os_error(code));
        }
        // SAFETY: the thread is created and suspended. `Logon` records the address of
        // the status, which is boxed and therefore does not move; `Resume` then lets
        // the thread run.
        unsafe {
            RThread_Logon(&thread.handle, &raw mut *thread.status);
            RThread_Resume(&thread.handle);
        }
        Ok(thread)
    }

    pub fn join(mut self) {
        // SAFETY: `Logon` completes the status when the thread ends, and nothing else
        // waits on it; the status outlives the wait because `self` owns it.
        unsafe { User_WaitForRequest(&raw mut *self.status) };
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        // SAFETY: `RThread::Close()` is `RHandleBase::Close()`, non-leaving and safe on
        // a handle that was never opened. Closing the handle does not end the thread.
        unsafe { RHandleBase_Close((&raw mut self.handle).cast::<RHandleBase>()) };
    }
}

/// Runs every thread-local destructor this thread owes, and then `std`'s own per-thread
/// cleanup. Nothing in the Symbian kernel does either.
pub fn drop_thread_locals() {
    // SAFETY: called only where the calling thread is finishing — the end of
    // `trampoline`, and `rt::symbian_start` after `main` has returned.
    unsafe { crate::sys::thread_local::key::run_dtors() };
    crate::rt::thread_cleanup();
    // SAFETY: as above; `thread_cleanup` may itself have left a thread-local set.
    unsafe { crate::sys::thread_local::key::run_dtors() };
}

/// EKA2 9.3 runs on one core and offers no call that says otherwise.
pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    Ok(NonZero::new(1).unwrap_or(NonZero::<usize>::MIN))
}

/// `RThread::Id` has not been observed, so there is no number to hand back.
pub fn current_os_id() -> Option<u64> {
    None
}

/// `User::After(0)` is the reschedule point experiment 72 used to make the emulator
/// interleave two threads; without it EKA2L1 never preempts a tight loop.
pub fn yield_now() {
    // SAFETY: a euser static that blocks the calling thread only and cannot leave.
    unsafe { User_After(0) };
}

/// `RThread::RenameMe` takes a descriptor and is unobserved; a Rust thread name stays
/// a Rust-side string, which is all `std::thread::Thread::name` promises.
pub fn set_name(_name: &CStr) {}

/// `User::After` takes a `TInt` of microseconds, so the longest single sleep is about
/// 35 minutes; a longer duration is slept in whole chunks of that.
pub fn sleep(duration: Duration) {
    let mut left = duration.as_micros();
    let step = i32::MAX as u128;
    while left > 0 {
        let now = left.min(step) as i32;
        // SAFETY: as `yield_now`.
        unsafe { User_After(now) };
        left -= now as u128;
    }
}
