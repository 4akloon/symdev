//! `TRequestStatus` and `User::WaitForRequest`: the two halves of a blocking Symbian
//! asynchronous call.
//!
//! Both are euser's, not esock's. They live in this module because the socket API is the
//! first thing in this SDK to need them; when a second subsystem does (step 73's active
//! scheduler, or `RTimer`), they move to [`crate::euser`] and this module re-exports them.
//!
//! # Why this is not an executor
//!
//! Every interesting `RSocket` operation takes a `TRequestStatus&` and completes later,
//! which looks like it demands a `CActive` and a `CActiveScheduler`. It does not. Symbian
//! itself spells the blocking form of an asynchronous call as the request followed by
//! `User::WaitForRequest`, which blocks the calling thread on the request semaphore until
//! the server completes the status. That is exactly `std::net`'s contract, so
//! `symbian_std::net` is built on this pair and step 73 adds the non-blocking path beside
//! it rather than underneath it.
//!
//! # The rule a caller must keep
//!
//! `User::WaitForRequest` waits on the *thread's* request semaphore, not on this
//! particular status: it returns when **any** request completes and then checks whether
//! this one did. So a thread may have at most one outstanding request while it blocks
//! here, and every `TRequestStatus` handed to a server must be waited for or cancelled
//! before the storage dies. `symbian_core::net` keeps that by never handing out a status
//! — each operation issues its request and waits before returning.

/// `KRequestPending` (`e32const.h` line 960: `const TInt KRequestPending=(-KMaxTInt);`):
/// the value in a `TRequestStatus` that means "the server still owns this request".
/// `-KMaxTInt` is `-0x7fffffff`, which is `i32::MIN + 1`.
pub const KREQUEST_PENDING: i32 = i32::MIN + 1;

/// The opaque `TRequestStatus` an export takes by `&`; only ever seen behind a pointer.
#[repr(C)]
pub struct TRequestStatus {
    _private: [u8; 0],
}

/// Storage for a `TRequestStatus`: `sizeof(TRequestStatus) == 8`, `alignof == 4`,
/// measured by compiling `return sizeof(TRequestStatus);` with the recorded GCCE argv.
///
/// The class is `e32cmn.h` line 2056: `TInt iStatus; TUint iFlags;`, every member
/// function inline and none of them leaving. So word 0 is the completion code the server
/// writes and word 1 is `iFlags` (`EActive = 1`, `ERequestPending = 2`), which euser and
/// the kernel own and this type never touches. Zeroed storage is `KErrNone`, so a caller sets
/// [`TRequestStatusStorage::set_pending`] before issuing the request — a request that is
/// rejected before the server sees it would otherwise read as success.
#[repr(C, align(4))]
pub struct TRequestStatusStorage {
    words: [i32; 2],
}

impl TRequestStatusStorage {
    /// Zeroed storage, which reads as `KErrNone` until [`Self::set_pending`] is called.
    pub const fn zeroed() -> Self {
        Self { words: [0; 2] }
    }

    /// Writes `KRequestPending` into `iStatus`, as every Symbian caller does immediately
    /// before issuing the request.
    pub const fn set_pending(&mut self) {
        self.words[0] = KREQUEST_PENDING;
    }

    /// The completion code the server wrote: `KErrNone` or a negative `TInt`.
    ///
    /// Meaningless until `User::WaitForRequest` has returned for this status; it is
    /// [`KREQUEST_PENDING`] while the request is outstanding.
    pub const fn code(&self) -> i32 {
        self.words[0]
    }

    /// Whether the server still owns the request.
    pub const fn is_pending(&self) -> bool {
        self.words[0] == KREQUEST_PENDING
    }

    /// The `TRequestStatus&` an asynchronous export expects.
    pub const fn as_request_status(&mut self) -> *mut TRequestStatus {
        (self as *mut Self).cast()
    }
}

unsafe extern "C" {
    /// `0000096c T _ZN4User14WaitForRequestER14TRequestStatus` — `User::WaitForRequest
    /// (TRequestStatus& aStatus)`, euser.dso.
    ///
    /// Blocks the calling thread on its request semaphore until `aStatus` is no longer
    /// `KRequestPending`. `e32std.h` declares it `IMPORT_C static void
    /// WaitForRequest(TRequestStatus&)` with no leave, so it is called directly.
    #[link_name = "_ZN4User14WaitForRequestER14TRequestStatus"]
    pub fn User_WaitForRequest(status: *mut TRequestStatus);
}
