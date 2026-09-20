//! `esock.dll` and `insock.dll` exports: the socket server session (`RSocketServ`), a
//! socket (`RSocket`), the name resolver (`RHostResolver`) and the address types
//! (`TSockAddr`, `TInetAddr`). Every mangled name is what `nm -D
//! epoc32/release/armv5/lib/<dll>.dso` prints and is cited next to the declaration.
//!
//! **None of it leaves, and that is checked rather than recalled.** `es_sock.h` declares
//! no leaving member on `RSocketServ`, `RSocket` or `RHostResolver`: every `IMPORT_C … L(`
//! in that header belongs to the `CSubConParameterSet` / `CSubConParameterFamily` /
//! `CSubConParameterBundle` / `CSubConNotificationEvent` family (lines 1204-1394), none of
//! which this SDK calls. `in_sock.h` declares no leaving member at all. So the whole
//! socket API is declared here and called directly, with `this` as argument 0 — the member
//! ABI observed in experiment 78 — and no C++ shim stands in the path (design spec §7).
//!
//! The one export on these classes that is *not* a C signature is
//! `RSocketServ::Version()`, which returns `TVersion` by value and is therefore sret
//! (rule 2 of `symrs_shim.h`). Nothing here needs it, so it is not declared.
//!
//! # Every operation is asynchronous in shape, and that does not make it async
//!
//! `Connect`, `Send`, `Recv`, `Read`, `Write`, `Accept` and `Shutdown` take a
//! `TRequestStatus&` and complete later. The **blocking** form of each — which is what
//! `std::net` is — is the pair `RSocket::Connect(addr, status); User::WaitForRequest
//! (status);` with no `CActive` and no scheduler. [`request`] holds that pair. Step 73's
//! executor will sit on these same declarations rather than replace them.
//!
//! # Sizes
//!
//! Measured by compiling `return sizeof(T);` with the recorded GCCE argv and reading the
//! immediate, the way experiments 78 and 79 did — note that the compiler emits
//! `movs rN,#k; lsls rN,#2` for a size above 255, so the immediate is a quarter of the
//! answer:
//!
//! | Type | `sizeof` | `alignof` |
//! |---|---|---|
//! | `TSockAddr` | 40 | 4 |
//! | `TInetAddr` | 40 | 4 |
//! | `RSocketServ` | 4 | 4 |
//! | `RSocket` | 8 | 4 |
//! | `RHostResolver` | 8 | 4 |
//! | `TRequestStatus` | 8 | 4 |
//! | `TNameRecord` | 564 | 8 |
//! | `TNameEntry` (`TPckgBuf<TNameRecord>`) | 576 | 8 |
mod pckgbuf;
mod request;
mod rhostresolver;
mod rsocket;
mod rsocketserv;
mod sockaddr;

pub use pckgbuf::{TPCKG_BUF_OFFSET_DATA, TPckgBuf_ctor, TSockXfrLengthStorage};
pub use request::{KREQUEST_PENDING, TRequestStatus, TRequestStatusStorage, User_WaitForRequest};
pub use rhostresolver::{
    RHostResolver, RHostResolver_Close, RHostResolver_GetByAddress, RHostResolver_GetByName,
    RHostResolver_Next, RHostResolver_Open, TNAME_ENTRY_OFFSET_ADDR, TNAME_ENTRY_OFFSET_DATA,
    TNAME_ENTRY_OFFSET_NAME, TNAME_RECORD_SIZE, TNameEntry, TNameEntryStorage,
};
pub use rsocket::{
    ESHUTDOWN_IMMEDIATE, ESHUTDOWN_NORMAL, ESHUTDOWN_STOP_INPUT, ESHUTDOWN_STOP_OUTPUT, RSocket,
    RSocket_Accept, RSocket_Bind, RSocket_Close, RSocket_Connect, RSocket_Listen,
    RSocket_LocalName, RSocket_Open, RSocket_OpenBlank, RSocket_Recv, RSocket_RecvFrom,
    RSocket_RecvOneOrMore, RSocket_RemoteName, RSocket_Send, RSocket_SendTo, RSocket_Shutdown,
};
pub use rsocketserv::{
    KESOCK_DEFAULT_MESSAGE_SLOTS, KPROTOCOL_INET_TCP, KPROTOCOL_INET_UDP, KSOCK_DATAGRAM,
    KSOCK_STREAM, RSocketServ, RSocketServ_Close, RSocketServ_Connect,
};
pub use sockaddr::{
    KAF_INET, KAF_INET6, KAF_UNSPEC, TInetAddr_Address, TInetAddr_ConvertToV4,
    TInetAddr_IsV4Mapped, TInetAddr_SetAddress, TInetAddr_ctor, TSockAddr, TSockAddr_Family,
    TSockAddr_Port, TSockAddr_SetFamily, TSockAddr_SetPort, TSockAddrStorage,
};
