//! `Socket`: one `RSocket`, and the operations `TcpStream`, `TcpListener` and
//! `UdpSocket` are each a view of.
//!
//! Nothing here leaves (`es_sock.h` declares no leaving member on `RSocket`), so there
//! is no C++ in the path. Everything that takes a `TRequestStatus&` goes through
//! `sys::pal::symbian::request::blocking`.

use super::addr::InetAddr;
use crate::sys::pal::symbian::request::blocking;
use super::session::with_session;
use crate::cell::UnsafeCell;
use crate::io;
use crate::sys::pal::symbian::des::{Bytes, BytesMut};
use symbian_sys::esock::{
    ESHUTDOWN_IMMEDIATE, ESHUTDOWN_NORMAL, ESHUTDOWN_STOP_INPUT, ESHUTDOWN_STOP_OUTPUT, KAF_INET,
    KPROTOCOL_INET_TCP, KPROTOCOL_INET_UDP, KSOCK_DATAGRAM, KSOCK_STREAM, RSocket, RSocket_Accept,
    RSocket_Bind, RSocket_Close, RSocket_Connect, RSocket_Listen, RSocket_LocalName, RSocket_Open,
    RSocket_OpenBlank, RSocket_RecvFrom, RSocket_RecvOneOrMore, RSocket_RemoteName, RSocket_Send,
    RSocket_SendTo, RSocket_Shutdown, TPckgBuf_ctor, TSockXfrLengthStorage,
};

/// `KErrEof` (`e32err.h`): what the stack completes a read with when the peer has
/// closed its end. Observed in step 74, where it first arrived as a failed
/// `read_to_end`.
const KERR_EOF: i32 = -25;

/// An open socket.
pub struct Socket {
    /// `std`'s socket methods take `&self` — two threads may read and write one
    /// `TcpStream` at once — while every `RSocket` member takes `this` as a `*mut`.
    inner: UnsafeCell<RSocket>,
}

// SAFETY: this is the same claim `sys::fs::symbian`'s `File` makes, and it is a claim
// about the *type system*, not about the platform: `std::net::TcpStream` is documented
// `Send + Sync` and a backend that was not would silently change the public API. The
// platform's own rule is narrower and is documented in `session`: a sub-session handle
// belongs to the thread that opened it, and the socket server answers `KErrBadHandle`
// to any other. Nothing here can enforce that, and nothing here pretends to; what this
// impl guarantees is only that the two handle words are plain data with no interior
// invariant Rust could break.
unsafe impl Send for Socket {}
unsafe impl Sync for Socket {}

impl Socket {
    const fn closed() -> Self {
        Self { inner: UnsafeCell::new(RSocket { session_handle: 0, sub_session_handle: 0 }) }
    }

    fn handle(&self) -> *mut RSocket {
        self.inner.get()
    }

    /// A TCP socket over IPv4.
    pub fn tcp() -> io::Result<Self> {
        Self::opened(KSOCK_STREAM, KPROTOCOL_INET_TCP)
    }

    /// A UDP socket over IPv4.
    pub fn udp() -> io::Result<Self> {
        Self::opened(KSOCK_DATAGRAM, KPROTOCOL_INET_UDP)
    }

    fn opened(sock_type: u32, protocol: u32) -> io::Result<Self> {
        let socket = Self::closed();
        with_session(|session| {
            // SAFETY: `this` in argument 0 per the observed member ABI, then the
            // session (borrowed mutably, as `RSocketServ&` is) and three scalars.
            // Non-leaving; on failure the handles stay zero and `Drop` is safe.
            check(unsafe {
                RSocket_Open(socket.handle(), session.as_server(), KAF_INET, sock_type, protocol)
            })
        })?;
        Ok(socket)
    }

    /// Connects and blocks until the stack answers (`RSocket::Connect`).
    pub fn connect(&self, addr: &mut InetAddr) -> io::Result<()> {
        let handle = self.handle();
        let addr = addr.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: `this` in argument 0; the address is a live `TInetAddr` borrowed
            // for the call (Symbian takes it non-`const` although it only reads it),
            // and the status is the live one `blocking` waits for immediately after.
            unsafe { RSocket_Connect(handle, addr, status) };
        })
        .map(|_| ())
    }

    /// `RSocket::Bind`, which is synchronous.
    pub fn bind(&self, addr: &mut InetAddr) -> io::Result<()> {
        // SAFETY: `this` in argument 0 and a live `TInetAddr`. Non-leaving; the failure
        // is the returned `TInt`.
        check(unsafe { RSocket_Bind(self.handle(), addr.as_sockaddr_mut()) })
    }

    /// `RSocket::Listen` with room for `backlog` pending connections.
    pub fn listen(&self, backlog: u32) -> io::Result<()> {
        // SAFETY: `this` in argument 0 and a scalar. Non-leaving.
        check(unsafe { RSocket_Listen(self.handle(), backlog) })
    }

    /// Waits for a connection (`RSocket::Accept`). The socket it moves the connection
    /// into must have been opened with `RSocket::Open(RSocketServ&)` and no protocol,
    /// which is what [`Socket::blank`] is; Symbian panics a client that hands it
    /// anything else.
    pub fn accept(&self) -> io::Result<Socket> {
        let accepted = Self::blank()?;
        let handle = self.handle();
        let blank = accepted.handle();
        blocking(|status| {
            // SAFETY: `this` in argument 0, then the blank socket (a live, open
            // `RSocket`) and the status `blocking` waits for. Non-leaving.
            unsafe { RSocket_Accept(handle, blank, status) };
        })?;
        Ok(accepted)
    }

    fn blank() -> io::Result<Self> {
        let socket = Self::closed();
        with_session(|session| {
            // SAFETY: as [`Self::opened`], with one fewer argument.
            check(unsafe { RSocket_OpenBlank(socket.handle(), session.as_server()) })
        })?;
        Ok(socket)
    }

    /// Sends all of `buf` (`RSocket::Send`). Symbian sends the whole descriptor or
    /// fails, so there is no short write and nothing for a caller to loop over.
    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let des = Bytes::new(buf)?;
        let handle = self.handle();
        let des = des.as_tdesc8();
        blocking(|status| {
            // SAFETY: `this` in argument 0; the descriptor is a real `TPtrC8` built by
            // euser over `buf`, which stays borrowed until this function returns — and
            // `blocking` does not return until the send has completed, so the stack is
            // never left holding a pointer into freed memory.
            unsafe { RSocket_Send(handle, des, 0, status) };
        })
        .map(|_| buf.len())
    }

    /// Reads whatever has arrived, up to `buf.len()` (`RSocket::RecvOneOrMore`).
    ///
    /// `RecvOneOrMore` and not `Recv`: `Recv` does not complete until the descriptor is
    /// full, which is not what `std::io::Read` promises. End of input arrives as
    /// `KErrEof` (-25) rather than as zero bytes — observed in step 74 — and is
    /// reported here as the bytes that *did* arrive, which is `0` when none did. That
    /// is exactly what `Read` means by end of stream, and no delivered byte is lost.
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut des = BytesMut::new(buf)?;
        let mut xfr = TSockXfrLengthStorage::zeroed();
        // SAFETY: zeroed storage of the measured `sizeof(TSockXfrLength)` (16),
        // 8-aligned as `TAlignedBuf8` requires, handed to euser's own exported
        // `TBufBase8(TInt,TInt)` with `sizeof(TInt)` for both — which is what the
        // inline `TPckgBuf<TInt>()` does. The descriptor header is built by euser and
        // never guessed here.
        unsafe {
            TPckgBuf_ctor(
                (&raw mut xfr).cast(),
                TSockXfrLengthStorage::payload_len(),
                TSockXfrLengthStorage::payload_len(),
            );
        }
        let handle = self.handle();
        let buf_des = des.as_tdes8();
        let xfr_des = xfr.as_tdes8();
        let outcome = blocking(|status| {
            // SAFETY: `this` in argument 0; `buf_des` is a real `TPtr8` over the
            // caller's slice with `iMaxLength` equal to its length, so every byte the
            // stack writes is inside it; `xfr_des` is the packaged `TInt` the client
            // stub carries the flags in and writes the count back to. Both outlive the
            // wait.
            unsafe { RSocket_RecvOneOrMore(handle, buf_des, 0, status, xfr_des) };
        });
        match outcome {
            Ok(_) => Ok(des.len()),
            Err(e) if e.raw_os_error() == Some(KERR_EOF) => Ok(des.len()),
            Err(e) => Err(e),
        }
    }

    /// One datagram to `addr` (`RSocket::SendTo`).
    pub fn send_to(&self, buf: &[u8], addr: &mut InetAddr) -> io::Result<usize> {
        let des = Bytes::new(buf)?;
        let handle = self.handle();
        let des = des.as_tdesc8();
        let addr = addr.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: as [`Self::send`], with a live `TInetAddr` in argument 2.
            unsafe { RSocket_SendTo(handle, des, addr, 0, status) };
        })
        .map(|_| buf.len())
    }

    /// One datagram and where it came from (`RSocket::RecvFrom`).
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, InetAddr)> {
        let mut des = BytesMut::new(buf)?;
        let mut from = InetAddr::blank();
        let handle = self.handle();
        let buf_des = des.as_tdes8();
        let from_addr = from.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: as [`Self::recv`]; `from_addr` is a constructed `TInetAddr` the
            // stack fills in, and it outlives the wait.
            unsafe { RSocket_RecvFrom(handle, buf_des, from_addr, 0, status) };
        })?;
        Ok((des.len(), from))
    }

    /// `RSocket::Shutdown`, blocking until it is done. `how` is one of the four
    /// `RSocket::TShutdown` values.
    pub fn shutdown(&self, how: i32) -> io::Result<()> {
        let handle = self.handle();
        blocking(|status| {
            // SAFETY: `this` in argument 0, then the enum as a scalar and the status.
            // Non-leaving.
            unsafe { RSocket_Shutdown(handle, how, status) };
        })
        .map(|_| ())
    }

    /// This socket's own address (`RSocket::LocalName`), which is how a listener bound
    /// to port 0 learns which port it got.
    pub fn local_addr(&self) -> InetAddr {
        let mut addr = InetAddr::blank();
        // SAFETY: `this` in argument 0 and a constructed `TInetAddr` the call fills in.
        // It returns `void`: there is no error path, and an unbound socket reads back
        // as `KAFUnspec`, which `InetAddr::to_socket` then refuses.
        unsafe { RSocket_LocalName(self.handle(), addr.as_sockaddr_mut()) };
        addr
    }

    /// The peer's address (`RSocket::RemoteName`).
    pub fn peer_addr(&self) -> InetAddr {
        let mut addr = InetAddr::blank();
        // SAFETY: as [`Self::local_addr`].
        unsafe { RSocket_RemoteName(self.handle(), addr.as_sockaddr_mut()) };
        addr
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        // SAFETY: `RSocket::Close` is a non-leaving member taking only `this`, and it
        // is safe on a socket that was never opened (both words are zero then).
        unsafe { RSocket_Close(self.inner.get()) };
    }
}

/// `std::net::Shutdown` as `RSocket::TShutdown`.
pub fn shutdown_how(how: crate::net::Shutdown) -> i32 {
    match how {
        crate::net::Shutdown::Read => ESHUTDOWN_STOP_INPUT,
        crate::net::Shutdown::Write => ESHUTDOWN_STOP_OUTPUT,
        crate::net::Shutdown::Both => ESHUTDOWN_NORMAL,
    }
}

/// `EImmediate`, the abortive close, which `std::net::Shutdown` has no name for.
pub const SHUTDOWN_IMMEDIATE: i32 = ESHUTDOWN_IMMEDIATE;

pub fn check(code: i32) -> io::Result<()> {
    if code == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(code)) }
}
