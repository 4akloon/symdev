//! `std::net` for Symbian: blocking TCP and UDP in `std`'s shape.
//!
//! ```ignore
//! use symbian_std::io::{Read, Write};
//! use symbian_std::net::TcpStream;
//!
//! let mut stream = TcpStream::connect(("example.test", 80))?;
//! stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
//! let mut buf = [0u8; 256];
//! let n = stream.read(&mut buf)?;
//! ```
//!
//! The addresses are `core::net`'s — [`SocketAddr`], [`IpAddr`], [`Ipv4Addr`] — not new
//! types with the same shape, so a crate that already speaks them needs no adapter.
//!
//! # There is no executor under this
//!
//! Every interesting `RSocket` operation takes a `TRequestStatus&` and completes later,
//! so a socket API on Symbian looks like it must be asynchronous. `std::net` is not, and
//! neither is this: the blocking form of a Symbian asynchronous call is the request
//! followed by `User::WaitForRequest`, which blocks the calling thread on its own request
//! semaphore. No `CActive`, no `CActiveScheduler`, no Rust executor exists below this
//! module. Step 73's async layer will sit *beside* it on the same `symbian-sys`
//! declarations.
//!
//! # What Symbian does that `std` has no word for
//!
//! §6a of the design spec: pretending would be a lie the first failure exposes.
//!
//! - **There is no access point here.** On a phone, reaching the internet means choosing
//!   an IAP — an `RConnection` — and `RSocket::Open` has an overload that takes one. This
//!   module uses the overload without it, so the socket server picks: on a device that is
//!   the default connection, or a dialog asking the user, or a failure if neither is
//!   configured; on EKA2L1 it is the host's own networking, which is why a success in the
//!   emulator says "the API works" and never "a device would connect". When choosing an
//!   access point becomes necessary it will be a Symbian-named type in a Symbian-named
//!   module, because there is nothing in `std` to call it.
//! - **`NetworkServices` is required**, and it is a build-time fact, not a runtime one:
//!   `in_sock.h` lines 66 and 74 say `@capability NetworkServices Required for opening
//!   'tcp' sockets. @ref RSocket::Open()`. `symdev build` refuses an image whose imports
//!   need a capability `[symbian] capabilities` does not grant, and names the manifest
//!   key. EKA2L1 does **not** enforce it — the same example passes there with an empty
//!   capability list — so the emulator cannot tell you that you forgot.
//! - **IPv6 is refused, not faked.** `TInetAddr` carries IPv6 and the stack often answers
//!   in `KAfInet6` with a v4-mapped address, which this module converts back. A genuine
//!   IPv6 address is [`ErrorKind::Unsupported`](crate::io::ErrorKind::Unsupported) with
//!   `KErrNotSupported` underneath: nothing here has ever sent a packet to one, and the
//!   scope-id and flow-label fields are not something to guess about.
//! - **One thread, one request.** `User::WaitForRequest` waits on the thread's request
//!   semaphore, so a thread can have one operation outstanding. Nothing in this SDK can
//!   spawn a thread yet (experiment 72: no atomics on ARMv5TE), so that is not a
//!   restriction anyone can hit today; it is why there are no non-blocking or
//!   timeout-carrying variants of these calls.
//!
//! # Where the shapes differ from `std`'s
//!
//! - **The methods take `&mut self`.** `std::net::TcpStream::read` takes `&self` because
//!   a POSIX descriptor is shared state behind the kernel. Here `RSocket::Recv` is a
//!   non-`const` member of the handle, and the borrow checker says so rather than hiding
//!   it — the same choice [`crate::fs::File`] made.
//! - **No `try_clone`, `set_nodelay`, `set_read_timeout`, `set_nonblocking`,
//!   `peek`, `connect_timeout`, `UdpSocket::connect`.** Each is a real Symbian call
//!   (`RSocket::SetOpt` mostly), and none of them has been exercised here; this SDK does
//!   not ship code it has not run.
//! - **`ToSocketAddrs::Iter` yields at most what the resolver answered**, and
//!   [`RHostResolver::Next`](symbian_core::net::HostResolver::next) is not walked: the
//!   first answer is used. Multiple answers are a `TODO` this module names rather than
//!   pretends about.
mod tcp_listener;
mod tcp_stream;
mod to_socket_addrs;
mod udp_socket;

pub use core::net::{
    AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
};

pub use tcp_listener::{Incoming, TcpListener};
pub use tcp_stream::TcpStream;
pub use to_socket_addrs::ToSocketAddrs;
pub use udp_socket::UdpSocket;

/// Which halves of a connection to close, as `std::net::Shutdown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Further reads are refused (`RSocket::EStopInput`).
    Read,
    /// Further writes are refused (`RSocket::EStopOutput`), which is what sends the FIN.
    Write,
    /// Both (`RSocket::ENormal`, the graceful close).
    Both,
}

impl Shutdown {
    pub(crate) const fn to_symbian(self) -> symbian_core::net::Shutdown {
        match self {
            Self::Read => symbian_core::net::Shutdown::Read,
            Self::Write => symbian_core::net::Shutdown::Write,
            Self::Both => symbian_core::net::Shutdown::Both,
        }
    }
}

/// `InetAddr` ← `SocketAddr`, refusing IPv6 rather than guessing at it.
pub(crate) fn to_inet(addr: SocketAddr) -> crate::io::Result<symbian_core::net::InetAddr> {
    match addr {
        // `u32::from(Ipv4Addr)` is the address in host order, which is what
        // `TInetAddr(TUint32, TUint)` takes.
        SocketAddr::V4(v4) => Ok(symbian_core::net::InetAddr::v4(
            u32::from(*v4.ip()),
            v4.port(),
        )),
        SocketAddr::V6(_) => Err(symbian_core::net::InetAddr::v6_unsupported().into()),
    }
}

/// `SocketAddr` ← `InetAddr`, converting a v4-mapped `KAfInet6` answer on the way.
pub(crate) fn from_inet(mut addr: symbian_core::net::InetAddr) -> crate::io::Result<SocketAddr> {
    let port = addr.port();
    let raw = addr.to_v4()?;
    Ok(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(raw), port)))
}
