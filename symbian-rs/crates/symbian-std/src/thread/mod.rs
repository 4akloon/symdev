//! `std::thread`'s shape over `RThread` (design spec §8, §11 step 72).
//!
//! ```ignore
//! let worker = thread::spawn(|| 6 * 7)?;
//! assert_eq!(worker.join()?, 42);
//! ```
//!
//! # One heap, and the crash that decides how a thread is created
//!
//! Symbian gives a new thread a heap. `RThread::Create` has two overloads: one takes a
//! minimum and maximum size and gives the thread a heap of its own, the other takes the
//! creator's `RAllocator*` so the two share one. The second is what Rust wants — a
//! `Box` allocated on one thread and dropped on another has to go back to the same heap
//! — and it is the one that does not work: **after a thread created that way exits, the
//! creator's very next allocation faults**, reading the heap chunk's own base address.
//! That is the `Access violation reading address 0x8000A4` experiment 72 recorded, and
//! experiment 80 reproduced it in pure C++ with no Rust in the picture, reduced it to
//! one `#define`, and showed `RAllocator::Open()` beforehand does not help.
//!
//! So [`spawn`] creates the thread with a heap of its own and the thread's **first
//! instruction** is `User::SwitchAllocator` onto the creator's heap. Measured: the
//! worker then allocates from the creator's heap, 400 interleaved alloc/free pairs on
//! each thread do not corrupt it, and the creator allocates happily after the join.
//! Concurrent access to that one heap is serialised by
//! [`symbian_alloc::serialise`](symbian_alloc), which `spawn` switches on before it
//! creates the first thread.
//!
//! # How this differs from `std::thread`
//!
//! - **`spawn` returns a `Result`.** `std::thread::spawn` panics if the OS refuses;
//!   library code here may not panic (CLAUDE.md), so the `RThread::Create` error comes
//!   back as an [`Error`]. It is `std::thread::Builder::spawn`'s shape under
//!   `std::thread::spawn`'s name.
//! - **Dropping a `JoinHandle` joins.** `std` detaches. Here the thread writes its
//!   result into a heap block the handle owns, so detaching would free memory the
//!   thread is still using. `Drop` therefore blocks until the thread has ended. There
//!   is no `detach`, because there is nothing safe for it to do yet.
//! - **A panicking thread is not a `Result::Err` you can inspect.** `panic = "abort"`
//!   ends the whole process, so `join` never sees a panicking thread; what it can see
//!   is a thread killed some other way, and that is an [`Error`] carrying the exit
//!   reason, not `Box<dyn Any>`.
//! # Thread-locals
//!
//! [`thread_local!`](crate::thread_local) and [`LocalKey`] are step 76's own half:
//! per-thread state over `UserSvr::DllTls`, one kernel call per access and no
//! compiler support at all. The values a thread initialised are dropped when it ends
//! — see [`local`](self) — which is the one thing [`spawn`] does after the closure
//! returns and before the thread exits.
//!
//! - **Not implemented:** `Builder` (no name, stack size or spawn options — every
//!   thread gets `KDefaultStackSize`, 8 kB), `Thread`/`ThreadId`/`current`, `park`,
//!   `scope`, `available_parallelism`. Nothing needed them, and each would be a guess.

use alloc::boxed::Box;
use core::ffi::c_void;
use core::time::Duration;

use symbian_core::{Buf16, DesC16, ErrorKind as SymKind};
use symbian_sys::euser::{RHandleBase_Close, User_After};
use symbian_sys::thread::{
    EOWNER_PROCESS, KDEFAULT_STACK_SIZE, RAllocator, RThread, RThread_CreateWithOwnHeap,
    RThread_ExitReason, RThread_ExitType, RThread_Logon, RThread_Resume, TRequestStatus,
    User_Allocator, User_SwitchAllocator, User_WaitForRequest,
};

use crate::io::{Error, Result};

mod local;
mod table;

pub use local::{AccessError, LocalKey, drop_thread_locals, live_thread_locals};

/// `EExitKill` = 0 of `TExitType` (`e32const.h` line 2241): the thread ran to the end
/// of its function, or `User::Exit` ended it.
const EEXIT_KILL: i32 = 0;

/// The worker's own heap, which it abandons for the creator's on its first
/// instruction. It has to be big enough for whatever the kernel puts there before our
/// code runs, and small because nothing else ever uses it.
const OWN_HEAP_MIN: i32 = 0x1000;
const OWN_HEAP_MAX: i32 = 0x10000;

/// The name every spawned thread gets. Symbian requires one and it must be unique
/// within the process only while the thread is alive; `std` has no equivalent, so it is
/// not part of the API.
const THREAD_NAME: &str = "symbian-std";

/// What crosses into the new thread: the heap to join, the work, and the result.
///
/// It lives on the heap and never moves, because `RThread::Logon` hands the kernel a
/// pointer to `status` and the worker writes `result` through a pointer of its own.
struct Packet<T> {
    status: TRequestStatus,
    heap: *mut RAllocator,
    body: Option<Box<dyn FnOnce() -> T + Send>>,
    result: Option<T>,
}

/// The `TThreadFunction` every spawned thread starts in.
///
/// # Safety
/// `arg` is the `Packet<T>` `spawn` leaked to this thread, alive until the handle is
/// joined, and this thread is the only one that touches `body` and `result`.
unsafe extern "C" fn trampoline<T>(arg: *mut c_void) -> i32 {
    let packet = arg.cast::<Packet<T>>();
    // SAFETY: the packet is live for the whole of this function. Switching the heap is
    // the FIRST thing, before anything can allocate or free, so that every allocation
    // this thread makes — including dropping the boxed closure below — goes to the
    // creator's heap and not to the one this thread is about to abandon.
    unsafe {
        User_SwitchAllocator((*packet).heap);
        if let Some(body) = (*packet).body.take() {
            (*packet).result = Some(body());
        }
    }
    // The thread is about to end, so this is where `Drop` runs for whatever it put in
    // a `thread_local!`. It happens after the closure's own result is stored, so a
    // destructor cannot change what `join` returns, and before the thread exits, so
    // the values are freed on the heap they were allocated from.
    drop_thread_locals();
    0
}

/// Runs `body` on a new thread.
///
/// The bounds are `std`'s: the closure and its result must be able to move between
/// threads and must not borrow anything that could go away.
pub fn spawn<F, T>(body: F) -> Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Before the first thread exists, and only from this thread: from here on every
    // allocation takes a lock, because two threads will be sharing one heap.
    // SAFETY: the process still has one thread — this one — which is exactly the
    // contract. A thread cannot have been created without coming through here.
    let code = unsafe { symbian_alloc::serialise_across_threads() };
    if code != 0 {
        return Err(Error::from_raw_os_error(code));
    }

    // SAFETY: `User::Allocator()` is a euser static returning this thread's heap by
    // reference; the pointer is only handed to the new thread's `SwitchAllocator`.
    let heap = unsafe { User_Allocator() };
    let mut packet = Box::new(Packet {
        status: TRequestStatus::new(),
        heap,
        body: Some(Box::new(body)),
        result: None,
    });

    let mut name = Buf16::<32>::new();
    if name.push_str(THREAD_NAME).is_err() {
        // Unreachable with the constant above, but a library here may not `unwrap`.
        return Err(Error::from_raw_os_error(SymKind::Overflow.code()));
    }
    let mut thread = RThread::null();
    // SAFETY: `name` is a live descriptor for the whole call; `trampoline::<T>` matches
    // `TThreadFunction`; the packet is boxed, so the pointer stays valid until the
    // handle is joined and dropped. The own-heap overload is deliberate — see the
    // module documentation for the access violation the allocator overload causes.
    let code = unsafe {
        RThread_CreateWithOwnHeap(
            &mut thread,
            name.as_tdesc16(),
            trampoline::<T>,
            KDEFAULT_STACK_SIZE,
            OWN_HEAP_MIN,
            OWN_HEAP_MAX,
            (&raw mut *packet).cast::<c_void>(),
            EOWNER_PROCESS,
        )
    };
    if code != 0 {
        return Err(Error::from_raw_os_error(code));
    }

    // SAFETY: the thread is created and suspended. `Logon` records the address of
    // `status`, which is inside the box and therefore does not move when the handle is
    // returned; `Resume` then lets the thread run.
    unsafe {
        RThread_Logon(&thread, &raw mut packet.status);
        RThread_Resume(&thread);
    }
    Ok(JoinHandle {
        thread,
        packet,
        joined: false,
    })
}

/// Blocks this thread for `duration`, as `std::thread::sleep`.
///
/// `User::After` takes a `TInt` of microseconds, so the longest single sleep is about
/// 35 minutes; a longer `duration` is slept in whole chunks of that.
pub fn sleep(duration: Duration) {
    let mut left = duration.as_micros();
    let step = i32::MAX as u128;
    while left > 0 {
        let now = left.min(step) as i32;
        // SAFETY: a euser static that blocks this thread only and cannot leave.
        unsafe { User_After(now) };
        left -= now as u128;
    }
}

/// Hands the processor to another runnable thread, as `std::thread::yield_now`.
///
/// `User::After(0)` is the reschedule point experiment 72 used to make the emulator
/// interleave two threads; without it EKA2L1 never preempts a tight loop.
pub fn yield_now() {
    // SAFETY: as `sleep`.
    unsafe { User_After(0) };
}

/// A handle to a spawned thread, as `std::thread::JoinHandle`.
///
/// Dropping it waits for the thread — see the module documentation.
pub struct JoinHandle<T> {
    thread: RThread,
    packet: Box<Packet<T>>,
    joined: bool,
}

// SAFETY: the handle owns the packet and only reads `result` after the thread has
// ended, so no two threads touch it at once; the kernel handle is `EOwnerProcess` and
// usable from any thread of the process.
unsafe impl<T: Send> Send for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    /// Waits for the thread to finish and returns what its closure returned.
    ///
    /// An `Err` means the thread did not run to the end of its closure: the code is its
    /// `RThread::ExitReason`.
    pub fn join(mut self) -> Result<T> {
        self.wait();
        // SAFETY: the thread has ended, so its exit type and reason are final; both are
        // non-leaving euser members with `this` as argument 0.
        let (exit_type, exit_reason) = unsafe {
            (
                RThread_ExitType(&self.thread),
                RThread_ExitReason(&self.thread),
            )
        };
        if exit_type != EEXIT_KILL || exit_reason != 0 {
            return Err(Error::from_raw_os_error(if exit_reason == 0 {
                SymKind::Abort.code()
            } else {
                exit_reason
            }));
        }
        // The thread wrote the result before it ended, and nothing else can take it.
        self.packet
            .result
            .take()
            .ok_or_else(|| Error::from_raw_os_error(SymKind::Abort.code()))
    }

    /// Whether the thread has already ended, as `std::thread::JoinHandle::is_finished`.
    ///
    /// `RThread::ExitType` is `EExitPending` (3) while it runs.
    pub fn is_finished(&self) -> bool {
        // SAFETY: reading the exit type of a live handle; non-leaving, `this` first.
        unsafe { RThread_ExitType(&self.thread) != EEXIT_PENDING }
    }

    /// Waits once, and remembers that it has.
    fn wait(&mut self) {
        if !self.joined {
            self.joined = true;
            // SAFETY: `Logon` was issued on this status before the thread was resumed,
            // and the status lives in the box this handle owns, so the kernel's pointer
            // is still valid. The request completes when the thread ends.
            unsafe { User_WaitForRequest(&raw mut self.packet.status) };
        }
    }
}

/// `EExitPending` = 3 of `TExitType` (`e32const.h` line 2241).
const EEXIT_PENDING: i32 = 3;

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        // The thread writes into the packet this handle owns, so it must be over before
        // the packet is freed. `std` detaches here; this cannot.
        self.wait();
        // SAFETY: the thread has ended and nothing else holds this handle;
        // `RThread::Close` is `RHandleBase::Close`, non-leaving.
        unsafe { RHandleBase_Close(&raw mut self.thread.base) };
    }
}
