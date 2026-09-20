//! `class RHostResolver`, name resolution, from
//! `nm -D epoc32/release/armv5/lib/esock.dso`.
//!
//! `es_sock.h` lines 849-906 declare no leaving member, so every entry point is called
//! directly with `this` as argument 0.
//!
//! **`GetByName` has a synchronous overload** — `TInt GetByName(const TDesC&,
//! TNameEntry&)` beside the `TRequestStatus&` one — so resolving a name needs no request
//! status and no wait at all. That is the overload declared here.

use crate::des::TDesC16;

use super::pckgbuf::TPCKG_BUF_OFFSET_DATA;
use super::rsocketserv::RSocketServ;
use super::sockaddr::{TSockAddr, TSockAddrStorage};

/// `sizeof(TNameRecord) == 564`, measured on the recorded GCCE argv. It is the
/// `aLength`/`aMaxLength` a `TNameEntry` is constructed with.
pub const TNAME_RECORD_SIZE: i32 = 564;

/// Where the `TNameRecord` begins inside a `TNameEntry`: 8, measured (see
/// [`super::pckgbuf`]).
pub const TNAME_ENTRY_OFFSET_DATA: usize = TPCKG_BUF_OFFSET_DATA;

/// Where `TNameRecord::iName` — a `THostName`, that is `TBuf<0x100>` (`es_sock.h` line
/// 553) — begins inside a `TNameEntry`. `iName` is at offset 0 of the record, measured.
pub const TNAME_ENTRY_OFFSET_NAME: usize = TNAME_ENTRY_OFFSET_DATA;

/// Where `TNameRecord::iAddr` — the answer — begins inside a `TNameEntry`. `iAddr` is at
/// offset **520** of the record, measured by compiling
/// `(TUint8*)&((TNameRecord*)0)->iAddr - (TUint8*)0` with the recorded GCCE argv.
pub const TNAME_ENTRY_OFFSET_ADDR: usize = TNAME_ENTRY_OFFSET_DATA + 520;

/// The opaque `TNameEntry` (`TPckgBuf<TNameRecord>`) an export takes by `&`.
#[repr(C)]
pub struct TNameEntry {
    _private: [u8; 0],
}

/// Storage for a `TNameEntry`: `sizeof == 576`, `alignof == 8`, measured.
///
/// Zeroed until [`TPckgBuf_ctor`] builds the descriptor header, which is the only way
/// this SDK ever writes one (the 8-bit type nibbles were never observed).
#[repr(C, align(8))]
pub struct TNameEntryStorage {
    bytes: [u8; 576],
}

impl TNameEntryStorage {
    /// Zeroed storage, ready for [`TPckgBuf_ctor`] with [`TNAME_RECORD_SIZE`] for both
    /// arguments.
    pub const fn zeroed() -> Self {
        Self { bytes: [0; 576] }
    }

    /// The start of the storage, for [`TPckgBuf_ctor`].
    pub const fn as_bytes_mut(&mut self) -> *mut u8 {
        (self as *mut Self).cast()
    }

    /// The `TNameEntry&` the resolver fills in.
    pub const fn as_name_entry(&mut self) -> *mut TNameEntry {
        (self as *mut Self).cast()
    }

    /// The resolved `TSockAddr` inside the record, for
    /// [`super::TSockAddr_Family`] and [`super::TInetAddr_Address`].
    ///
    /// Reading it is only meaningful after a `GetByName` or `GetByAddress` returned
    /// `KErrNone`; before that it is the zeroed storage.
    pub const fn addr(&self) -> *const TSockAddrStorage {
        // SAFETY: `TNAME_ENTRY_OFFSET_ADDR` (528) plus `sizeof(TSockAddr)` (40) is 568,
        // inside the 576 bytes this type owns, and the whole storage is 8-aligned so
        // offset 528 is 4-aligned as `TSockAddr` requires. Both numbers were measured
        // with a compile probe on the recorded GCCE argv, not derived.
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(TNAME_ENTRY_OFFSET_ADDR)
        }
        .cast()
    }

    /// The same address, mutably, for `TInetAddr::ConvertToV4` on an answer the stack
    /// returned in `KAfInet6` form.
    pub const fn addr_mut(&mut self) -> *mut TSockAddrStorage {
        // SAFETY: as [`Self::addr`].
        unsafe {
            (self as *mut Self)
                .cast::<u8>()
                .add(TNAME_ENTRY_OFFSET_ADDR)
        }
        .cast()
    }
}

/// `RHostResolver` is `RSubSessionBase`: two handle words —
/// `sizeof(RHostResolver) == 8`, measured.
#[repr(C)]
pub struct RHostResolver {
    pub session_handle: i32,
    pub sub_session_handle: i32,
}

unsafe extern "C" {
    /// `000001dc T _ZN13RHostResolver4OpenER11RSocketServjj` — `RHostResolver::Open
    /// (RSocketServ&, TUint anAddrFamily, TUint aProtocol)`.
    ///
    /// The overload **without** an `RConnection`, for the same §6a reason as
    /// [`super::RSocket_Open`]: an access point has no `std` name.
    #[link_name = "_ZN13RHostResolver4OpenER11RSocketServjj"]
    pub fn RHostResolver_Open(
        this: *mut RHostResolver,
        server: *mut RSocketServ,
        addr_family: u32,
        protocol: u32,
    ) -> i32;

    /// `000001f4 T _ZN13RHostResolver9GetByNameERK7TDesC16R8TPckgBufI11TNameRecordE` —
    /// `TInt RHostResolver::GetByName(const TDesC& aName, TNameEntry& aResult)`: the
    /// **synchronous** overload. `KErrNotFound` for a name that does not resolve.
    #[link_name = "_ZN13RHostResolver9GetByNameERK7TDesC16R8TPckgBufI11TNameRecordE"]
    pub fn RHostResolver_GetByName(
        this: *mut RHostResolver,
        name: *const TDesC16,
        result: *mut TNameEntry,
    ) -> i32;

    /// `000001c4 T _ZN13RHostResolver12GetByAddressERK9TSockAddrR8TPckgBufI11TNameRecordE`
    /// — `TInt RHostResolver::GetByAddress(const TSockAddr&, TNameEntry&)`: the reverse
    /// lookup, synchronous.
    #[link_name = "_ZN13RHostResolver12GetByAddressERK9TSockAddrR8TPckgBufI11TNameRecordE"]
    pub fn RHostResolver_GetByAddress(
        this: *mut RHostResolver,
        addr: *const TSockAddr,
        result: *mut TNameEntry,
    ) -> i32;

    /// `000001d4 T _ZN13RHostResolver4NextER8TPckgBufI11TNameRecordE` — `TInt
    /// RHostResolver::Next(TNameEntry& aResult)`: the next answer for the same
    /// question, `KErrEof` when there are no more.
    #[link_name = "_ZN13RHostResolver4NextER8TPckgBufI11TNameRecordE"]
    pub fn RHostResolver_Next(this: *mut RHostResolver, result: *mut TNameEntry) -> i32;

    /// `000001e4 T _ZN13RHostResolver5CloseEv` — `RHostResolver::Close()`.
    #[link_name = "_ZN13RHostResolver5CloseEv"]
    pub fn RHostResolver_Close(this: *mut RHostResolver);
}
