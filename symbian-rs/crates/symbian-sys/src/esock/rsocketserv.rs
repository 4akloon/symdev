//! `class RSocketServ`, the session with the socket server, from
//! `nm -D epoc32/release/armv5/lib/esock.dso`.
//!
//! `es_sock.h` lines 671-717 declare no leaving member, so both entry points this SDK
//! needs are called directly with `this` as argument 0.

/// `KESockDefaultMessageSlots` (`es_sock.h` line 53): the default `aMessageSlots` of
/// `RSocketServ::Connect`, which is the value the one-argument call supplies. Declared
/// explicitly because a default argument is a compile-time C++ thing that does not
/// survive into an `extern "C"` declaration.
pub const KESOCK_DEFAULT_MESSAGE_SLOTS: u32 = 8;

/// `KSockStream` (`es_sock.h` line 241): a reliable byte stream — TCP.
pub const KSOCK_STREAM: u32 = 1;

/// `KSockDatagram` (`es_sock.h` line 243): a message socket — UDP.
pub const KSOCK_DATAGRAM: u32 = 2;

/// `KProtocolInetTcp` (`in_sock.h` line 69).
pub const KPROTOCOL_INET_TCP: u32 = 6;

/// `KProtocolInetUdp` (`in_sock.h` line 77).
pub const KPROTOCOL_INET_UDP: u32 = 17;

/// `RSocketServ` is `RSessionBase` is `RHandleBase`: one handle word.
/// `sizeof(RSocketServ) == 4`, measured by compiling `return sizeof(RSocketServ);` with
/// the recorded GCCE argv. A default-constructed session is that word zero.
#[repr(C)]
pub struct RSocketServ {
    pub handle: i32,
}

unsafe extern "C" {
    /// `00000140 T _ZN11RSocketServ7ConnectEj` — `RSocketServ::Connect(TUint
    /// aMessageSlots)`: opens the IPC channel to the socket server.
    ///
    /// This is where a missing `NetworkServices` capability shows up, as
    /// `KErrPermissionDenied` (-46): the server's connection policy is checked before
    /// any socket exists.
    #[link_name = "_ZN11RSocketServ7ConnectEj"]
    pub fn RSocketServ_Connect(this: *mut RSocketServ, message_slots: u32) -> i32;

    /// `000001dc T _ZN11RHandleBase5CloseEv` — `RHandleBase::Close()`, euser.dso.
    /// `RSocketServ` declares no `Close` of its own
    /// (`es_sock.h` line 674 says so in as many words), so the base's is the one a C++
    /// caller reaches and the one the SDK calls. Closing the session closes every
    /// sub-session opened from it.
    #[link_name = "_ZN11RHandleBase5CloseEv"]
    pub fn RSocketServ_Close(this: *mut RSocketServ);
}
