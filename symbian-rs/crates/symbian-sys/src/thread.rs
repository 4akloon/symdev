//! `RThread`, `TRequestStatus` and the euser exports a thread needs, from
//! `nm -D epoc32/release/armv5/lib/euser.dso`.
//!
//! `RThread`'s members are non-static and non-virtual, so `this` is argument 0 under
//! the ordinary AAPCS assignment (observed in experiment 78); none of them leaves, so
//! none of them needs the C++ shim.

use crate::des::TDesC16;
use crate::euser::RHandleBase;

/// `typedef TInt (*TThreadFunction)(TAny*);` — `e32const.h` line 2734.
pub type TThreadFunction = unsafe extern "C" fn(*mut core::ffi::c_void) -> i32;

/// `const TInt KDefaultStackSize=0x2000;` — `e32const.h` line 513.
pub const KDEFAULT_STACK_SIZE: i32 = 0x2000;

/// `const TInt KMinHeapSize=0x100;` — `e32const.h` line 458.
pub const KMIN_HEAP_SIZE: i32 = 0x100;

/// `EOwnerProcess` of `TOwnerType` (`e32const.h` line 2085): the **first** enumerator,
/// so its value is 0, not 1 — the handle belongs to the process and any thread of it
/// may use and close it.
pub const EOWNER_PROCESS: i32 = 0;

/// `TRequestStatus` — `e32cmn.h` line 2097: `TInt iStatus; TUint iFlags;`, 8 bytes.
///
/// `KRequestPending` is `-0x7fffffff`, which is what a request is set to while it is
/// outstanding; the completion writes the result over it.
#[repr(C)]
pub struct TRequestStatus {
    pub status: i32,
    pub flags: u32,
}

impl TRequestStatus {
    /// `const TInt KRequestPending=(-KMaxTInt);` — `e32const.h` line 960.
    pub const PENDING: i32 = -0x7fff_ffff;

    /// A status that has not been handed to anything yet.
    pub const fn new() -> Self {
        Self {
            status: Self::PENDING,
            flags: 0,
        }
    }
}

impl Default for TRequestStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// `RThread` — `e32std.h` line 3522, one `TInt iHandle` inherited from `RHandleBase`
/// and nothing of its own.
#[repr(C)]
pub struct RThread {
    pub base: RHandleBase,
}

impl RThread {
    /// A handle that names no thread. `RThread`'s own default constructor sets the
    /// handle to `KCurrentThreadHandle`; this is the null handle instead, because the
    /// SDK only ever fills it in through `Create`.
    pub const fn null() -> Self {
        Self {
            base: RHandleBase { handle: 0 },
        }
    }
}

/// `RAllocator` (`e32cmn.h` line 2520) is only ever handled as an opaque pointer here:
/// `User::Allocator()` returns one and `RThread::Create` takes one.
#[repr(C)]
pub struct RAllocator {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    /// `00001214 T _ZN7RThread6CreateERK7TDesC16PFiPvEiP10RAllocatorS3_10TOwnerType` —
    /// `RThread::Create(const TDesC&, TThreadFunction, TInt aStackSize, RAllocator*
    /// aHeap, TAny* aPtr, TOwnerType)`.
    ///
    /// The overload that takes an **allocator** rather than a heap size pair. That is
    /// the one a Rust thread needs: the other overload gives the new thread a heap of
    /// its own, and a `Box` or an `Arc` allocated on one thread and dropped on another
    /// would then be returned to the wrong heap.
    #[link_name = "_ZN7RThread6CreateERK7TDesC16PFiPvEiP10RAllocatorS3_10TOwnerType"]
    pub fn RThread_CreateWithAllocator(
        this: *mut RThread,
        name: *const TDesC16,
        function: TThreadFunction,
        stack_size: i32,
        heap: *mut RAllocator,
        ptr: *mut core::ffi::c_void,
        owner: i32,
    ) -> i32;

    /// `00001218 T _ZN7RThread6CreateERK7TDesC16PFiPvEiiiS3_10TOwnerType` —
    /// `RThread::Create(const TDesC&, TThreadFunction, TInt aStackSize, TInt
    /// aHeapMinSize, TInt aHeapMaxSize, TAny* aPtr, TOwnerType)`: the overload that
    /// gives the new thread a heap **of its own**. Kept for the record and for
    /// diagnosis; a Rust thread cannot use it, because a `Box` or an `Arc` allocated on
    /// one thread and dropped on another would go back to the wrong heap.
    #[link_name = "_ZN7RThread6CreateERK7TDesC16PFiPvEiiiS3_10TOwnerType"]
    pub fn RThread_CreateWithOwnHeap(
        this: *mut RThread,
        name: *const TDesC16,
        function: TThreadFunction,
        stack_size: i32,
        heap_min: i32,
        heap_max: i32,
        ptr: *mut core::ffi::c_void,
        owner: i32,
    ) -> i32;

    /// `00001c08 T _ZNK7RThread6ResumeEv` — `RThread::Resume() const`; a thread is
    /// created suspended.
    #[link_name = "_ZNK7RThread6ResumeEv"]
    pub fn RThread_Resume(this: *const RThread);

    /// `00001c04 T _ZNK7RThread5LogonER14TRequestStatus` —
    /// `RThread::Logon(TRequestStatus&) const`: completes `status` with the thread's
    /// exit reason when it dies. This is how a join is built.
    #[link_name = "_ZNK7RThread5LogonER14TRequestStatus"]
    pub fn RThread_Logon(this: *const RThread, status: *mut TRequestStatus);

    /// `00001c18 T _ZNK7RThread8ExitTypeEv` — `RThread::ExitType() const`.
    /// `EExitKill` = 0, `EExitTerminate` = 1, `EExitPanic` = 2, `EExitPending` = 3
    /// (`TExitType`, `e32const.h`).
    #[link_name = "_ZNK7RThread8ExitTypeEv"]
    pub fn RThread_ExitType(this: *const RThread) -> i32;

    /// `00001bd0 T _ZNK7RThread10ExitReasonEv` — `RThread::ExitReason() const`.
    #[link_name = "_ZNK7RThread10ExitReasonEv"]
    pub fn RThread_ExitReason(this: *const RThread) -> i32;

    /// `0000096c T _ZN4User14WaitForRequestER14TRequestStatus` —
    /// `User::WaitForRequest(TRequestStatus&)`: blocks this thread until `status` is
    /// completed.
    #[link_name = "_ZN4User14WaitForRequestER14TRequestStatus"]
    pub fn User_WaitForRequest(status: *mut TRequestStatus);

    /// `00000a60 T _ZN4User9AllocatorEv` — `User::Allocator()`, the calling thread's
    /// heap, returned by reference (a pointer under the ABI).
    #[link_name = "_ZN4User9AllocatorEv"]
    pub fn User_Allocator() -> *mut RAllocator;

    /// `00000078 T _ZN10RAllocator4OpenEv` — `RAllocator::Open()`: one more reference
    /// to the heap. Returns a system error code.
    #[link_name = "_ZN10RAllocator4OpenEv"]
    pub fn RAllocator_Open(this: *mut RAllocator) -> i32;

    /// `000009a4 T _ZN4User15SwitchAllocatorEP10RAllocator` —
    /// `User::SwitchAllocator(RAllocator*)`: makes `allocator` the **calling thread's**
    /// heap and returns the one it replaced.
    ///
    /// This is how a spawned thread joins the heap its creator is using. It has to be
    /// done from inside the new thread, and the thread has to have been created with
    /// the own-heap overload, because passing the creator's allocator to
    /// `RThread::Create` makes the worker's exit take the creator's heap down with it
    /// (observed; see `docs/research/eka2-concurrency.md`).
    #[link_name = "_ZN4User15SwitchAllocatorEP10RAllocator"]
    pub fn User_SwitchAllocator(allocator: *mut RAllocator) -> *mut RAllocator;

    /// `0000007c T _ZN10RAllocator5CloseEv` — `RAllocator::Close()`: one reference
    /// fewer; the last one destroys the heap's chunk.
    #[link_name = "_ZN10RAllocator5CloseEv"]
    pub fn RAllocator_Close(this: *mut RAllocator);
}

unsafe extern "C" {
    /// `00001618 T _ZN9RFastLock11CreateLocalE10TOwnerType` —
    /// `RFastLock::CreateLocal(TOwnerType)`. Non-leaving; returns a system error code.
    #[link_name = "_ZN9RFastLock11CreateLocalE10TOwnerType"]
    pub fn RFastLock_CreateLocal(this: *mut RFastLock, owner: i32) -> i32;

    /// `0000161c T _ZN9RFastLock4WaitEv` — `RFastLock::Wait()`. **Not recursive**: a
    /// second `Wait` from the thread that already holds it blocks for ever (observed,
    /// experiment 72), which is exactly Rust's `Mutex` contract.
    #[link_name = "_ZN9RFastLock4WaitEv"]
    pub fn RFastLock_Wait(this: *mut RFastLock);

    /// `00001620 T _ZN9RFastLock6SignalEv` — `RFastLock::Signal()`.
    #[link_name = "_ZN9RFastLock6SignalEv"]
    pub fn RFastLock_Signal(this: *mut RFastLock);
}

/// `RFastLock` — `e32cmn.h` line 2437: `RSemaphore`'s `TInt iHandle` plus its own
/// `TInt iCount`, 8 bytes (measured, experiment 72). Process-local, created only by
/// `CreateLocal`.
#[repr(C)]
pub struct RFastLock {
    pub base: RHandleBase,
    pub count: i32,
}

impl RFastLock {
    /// An unopened lock. `RFastLock`'s own constructor is
    /// `inline RFastLock() : iCount(0)` over `RHandleBase`'s `iHandle(0)`
    /// (`e32cmn.inl` line 3158), so all-zero is exactly what C++ would build.
    pub const fn null() -> Self {
        Self {
            base: RHandleBase { handle: 0 },
            count: 0,
        }
    }
}

unsafe extern "C" {
    /// `000000fc T _ZN10RSemaphore11CreateLocalEi10TOwnerType` —
    /// `RSemaphore::CreateLocal(TInt aCount, TOwnerType)`.
    #[link_name = "_ZN10RSemaphore11CreateLocalEi10TOwnerType"]
    pub fn RSemaphore_CreateLocal(this: *mut RSemaphore, count: i32, owner: i32) -> i32;

    /// `0000010c T _ZN10RSemaphore4WaitEi` — `RSemaphore::Wait(TInt aTimeout)`, the
    /// timeout in microseconds. **The only timed wait in the SDK**: neither `RFastLock`
    /// nor `RMutex` has one. Returns `KErrNone`, or `KErrTimedOut` (−33) when the
    /// timeout ran out first (observed, experiment 72).
    #[link_name = "_ZN10RSemaphore4WaitEi"]
    pub fn RSemaphore_WaitTimeout(this: *mut RSemaphore, timeout_micros: i32) -> i32;

    /// `00000110 T _ZN10RSemaphore4WaitEv` — `RSemaphore::Wait()`, untimed.
    #[link_name = "_ZN10RSemaphore4WaitEv"]
    pub fn RSemaphore_Wait(this: *mut RSemaphore);

    /// `00000118 T _ZN10RSemaphore6SignalEv` — `RSemaphore::Signal()`.
    #[link_name = "_ZN10RSemaphore6SignalEv"]
    pub fn RSemaphore_Signal(this: *mut RSemaphore);
}

/// `RSemaphore` — `e32cmn.h` line 2408: `RHandleBase`'s `TInt iHandle` and nothing
/// else, 4 bytes (measured, experiment 72).
#[repr(C)]
pub struct RSemaphore {
    pub base: RHandleBase,
}

impl RSemaphore {
    /// An unopened semaphore, as a value rather than a call: `std`'s platform layer
    /// needs it inside a `const fn` that must stay const-stable, and a `const fn` of
    /// another crate cannot be called from one.
    pub const NULL: Self = Self {
        base: RHandleBase { handle: 0 },
    };

    /// An unopened semaphore.
    pub const fn null() -> Self {
        Self::NULL
    }
}
