//! `class RSocket`, one endpoint of a connection, from
//! `nm -D epoc32/release/armv5/lib/esock.dso`.
//!
//! `es_sock.h` lines 725-818 declare no leaving member, so every entry point below is
//! called directly with `this` as argument 0 under the member ABI of experiment 78.
//!
//! The operations split in two, and the split is the whole shape of this module:
//!
//! - **`TInt` now** — `Open`, `Bind`, `Listen`, `Close`, `LocalName`, `RemoteName`. These
//!   are ordinary calls that answer immediately.
//! - **`TRequestStatus&` later** — `Connect`, `Send`, `Recv`, `SendTo`, `RecvFrom`,
//!   `Accept`, `Shutdown`. These return `void`; the answer arrives in the status. The
//!   blocking form is the call followed by [`super::User_WaitForRequest`], which is what
//!   `std::net` means and needs no scheduler.
//!
//! The `TSockXfrLength` overloads of `Send` and `Recv` are **not** declared: the
//! descriptor's own length after the call says the same thing and is already readable
//! through [`crate::des8::TPtr8Storage::length`]. `RecvOneOrMore` is the exception, and
//! not by choice — `es_sock.h` line 776 gives it no overload without one.

use crate::des8::{TDes8, TDesC8};

use super::request::TRequestStatus;
use super::rsocketserv::RSocketServ;
use super::sockaddr::TSockAddr;

/// `RSocket::ENormal` (`es_sock.h` line 751): complete when input and output have
/// stopped — the graceful close, `std`'s `Shutdown::Both`.
pub const ESHUTDOWN_NORMAL: i32 = 0;
/// `RSocket::EStopInput` (line 753): stop reading, complete when output stops.
pub const ESHUTDOWN_STOP_INPUT: i32 = 1;
/// `RSocket::EStopOutput` (line 755): stop writing, complete when input stops.
pub const ESHUTDOWN_STOP_OUTPUT: i32 = 2;
/// `RSocket::EImmediate` (line 757): abortive close.
pub const ESHUTDOWN_IMMEDIATE: i32 = 3;

/// `RSocket` is `RCommsSubSession` is `RSubSessionBase`: the session handle and the
/// sub-session handle, two words — `sizeof(RSocket) == 8`, measured by compiling
/// `return sizeof(RSocket);` with the recorded GCCE argv. A default-constructed socket
/// is both words zero.
#[repr(C)]
pub struct RSocket {
    pub session_handle: i32,
    pub sub_session_handle: i32,
}

unsafe extern "C" {
    /// `00000348 T _ZN7RSocket4OpenER11RSocketServjjj` — `RSocket::Open(RSocketServ&,
    /// TUint anAddrFamily, TUint aSockType, TUint aProtocol)`.
    ///
    /// This is the overload with **no** `RConnection`, which is the one that matters for
    /// §6a: naming an `RConnection` is naming an access point, and there is no `std`
    /// concept for that. See the crate documentation of `symbian_std::net`.
    #[link_name = "_ZN7RSocket4OpenER11RSocketServjjj"]
    pub fn RSocket_Open(
        this: *mut RSocket,
        server: *mut RSocketServ,
        addr_family: u32,
        sock_type: u32,
        protocol: u32,
    ) -> i32;

    /// `00000340 T _ZN7RSocket4OpenER11RSocketServ` — `RSocket::Open(RSocketServ&)`:
    /// opens a socket with no protocol, which is what `RSocket::Accept` needs as its
    /// blank destination.
    #[link_name = "_ZN7RSocket4OpenER11RSocketServ"]
    pub fn RSocket_OpenBlank(this: *mut RSocket, server: *mut RSocketServ) -> i32;

    /// `00000368 T _ZN7RSocket5CloseEv` — `RSocket::Close()`. Safe on a socket that was
    /// never opened.
    #[link_name = "_ZN7RSocket5CloseEv"]
    pub fn RSocket_Close(this: *mut RSocket);

    /// `00000334 T _ZN7RSocket4BindER9TSockAddr` — `RSocket::Bind(TSockAddr& anAddr)`.
    /// The address is taken non-`const` although the call only reads it.
    #[link_name = "_ZN7RSocket4BindER9TSockAddr"]
    pub fn RSocket_Bind(this: *mut RSocket, addr: *mut TSockAddr) -> i32;

    /// `00000384 T _ZN7RSocket6ListenEj` — `RSocket::Listen(TUint qSize)`.
    #[link_name = "_ZN7RSocket6ListenEj"]
    pub fn RSocket_Listen(this: *mut RSocket, queue_size: u32) -> i32;

    /// `000003bc T _ZN7RSocket9LocalNameER9TSockAddr` — `RSocket::LocalName(TSockAddr&)`:
    /// fills in this socket's own address. Returns `void`: there is no error path.
    #[link_name = "_ZN7RSocket9LocalNameER9TSockAddr"]
    pub fn RSocket_LocalName(this: *mut RSocket, addr: *mut TSockAddr);

    /// `00000314 T _ZN7RSocket10RemoteNameER9TSockAddr` — `RSocket::RemoteName
    /// (TSockAddr&)`: fills in the peer's address. Returns `void`.
    #[link_name = "_ZN7RSocket10RemoteNameER9TSockAddr"]
    pub fn RSocket_RemoteName(this: *mut RSocket, addr: *mut TSockAddr);

    /// `0000039c T _ZN7RSocket7ConnectER9TSockAddrR14TRequestStatus` —
    /// `RSocket::Connect(TSockAddr& anAddr, TRequestStatus& aStatus)`.
    #[link_name = "_ZN7RSocket7ConnectER9TSockAddrR14TRequestStatus"]
    pub fn RSocket_Connect(this: *mut RSocket, addr: *mut TSockAddr, status: *mut TRequestStatus);

    /// `00000360 T _ZN7RSocket4SendERK6TDesC8jR14TRequestStatus` — `RSocket::Send(const
    /// TDesC8& aDesc, TUint someFlags, TRequestStatus& aStatus)`: sends the whole
    /// descriptor or completes with an error.
    #[link_name = "_ZN7RSocket4SendERK6TDesC8jR14TRequestStatus"]
    pub fn RSocket_Send(
        this: *mut RSocket,
        desc: *const TDesC8,
        flags: u32,
        status: *mut TRequestStatus,
    );

    /// `00000358 T _ZN7RSocket4RecvER5TDes8jR14TRequestStatus` — `RSocket::Recv(TDes8&
    /// aDesc, TUint flags, TRequestStatus& aStatus)`.
    ///
    /// **`Recv` waits for the descriptor to be filled to `iMaxLength`**, which is not
    /// what `std::io::Read` means; `RecvOneOrMore` is the one that returns as soon as
    /// any bytes arrive. `symbian_core::net` uses `RecvOneOrMore` for reading and this
    /// one only where a caller asked for an exact count.
    #[link_name = "_ZN7RSocket4RecvER5TDes8jR14TRequestStatus"]
    pub fn RSocket_Recv(
        this: *mut RSocket,
        desc: *mut TDes8,
        flags: u32,
        status: *mut TRequestStatus,
    );

    /// `0000032c T _ZN7RSocket13RecvOneOrMoreER5TDes8jR14TRequestStatusR8TPckgBufIiE` —
    /// `RSocket::RecvOneOrMore(TDes8& aDesc, TUint flags, TRequestStatus& aStatus,
    /// TSockXfrLength& aLen)`: completes as soon as **any** bytes have arrived, which is
    /// `std::io::Read`'s contract and not [`RSocket_Recv`]'s. There is no overload
    /// without the `TSockXfrLength`, so this is the one call in the module that needs
    /// [`super::TSockXfrLengthStorage`].
    #[link_name = "_ZN7RSocket13RecvOneOrMoreER5TDes8jR14TRequestStatusR8TPckgBufIiE"]
    pub fn RSocket_RecvOneOrMore(
        this: *mut RSocket,
        desc: *mut TDes8,
        flags: u32,
        status: *mut TRequestStatus,
        len: *mut TDes8,
    );

    /// `0000038c T _ZN7RSocket6SendToERK6TDesC8R9TSockAddrjR14TRequestStatus` —
    /// `RSocket::SendTo(const TDesC8&, TSockAddr&, TUint flags, TRequestStatus&)`.
    #[link_name = "_ZN7RSocket6SendToERK6TDesC8R9TSockAddrjR14TRequestStatus"]
    pub fn RSocket_SendTo(
        this: *mut RSocket,
        desc: *const TDesC8,
        addr: *mut TSockAddr,
        flags: u32,
        status: *mut TRequestStatus,
    );

    /// `000003a4 T _ZN7RSocket8RecvFromER5TDes8R9TSockAddrjR14TRequestStatus` —
    /// `RSocket::RecvFrom(TDes8&, TSockAddr&, TUint flags, TRequestStatus&)`: one
    /// datagram, with the sender's address filled in.
    #[link_name = "_ZN7RSocket8RecvFromER5TDes8R9TSockAddrjR14TRequestStatus"]
    pub fn RSocket_RecvFrom(
        this: *mut RSocket,
        desc: *mut TDes8,
        addr: *mut TSockAddr,
        flags: u32,
        status: *mut TRequestStatus,
    );

    /// `00000374 T _ZN7RSocket6AcceptERS_R14TRequestStatus` — `RSocket::Accept(RSocket&
    /// aBlankSocket, TRequestStatus& aStatus)`. `aBlankSocket` must already be open
    /// through [`RSocket_OpenBlank`] on the same session.
    #[link_name = "_ZN7RSocket6AcceptERS_R14TRequestStatus"]
    pub fn RSocket_Accept(this: *mut RSocket, blank: *mut RSocket, status: *mut TRequestStatus);

    /// `000003ac T _ZN7RSocket8ShutdownENS_9TShutdownER14TRequestStatus` —
    /// `RSocket::Shutdown(TShutdown aHow, TRequestStatus& aStatus)`. The nested enum is
    /// passed as an `int` (`NS_9TShutdownE` in the mangling, an ordinary scalar
    /// argument).
    #[link_name = "_ZN7RSocket8ShutdownENS_9TShutdownER14TRequestStatus"]
    pub fn RSocket_Shutdown(this: *mut RSocket, how: i32, status: *mut TRequestStatus);
}
