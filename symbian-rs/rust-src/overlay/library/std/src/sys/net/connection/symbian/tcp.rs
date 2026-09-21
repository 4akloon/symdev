//! `TcpStream` and `TcpListener` over `RSocket`.
//!
//! What is real is what step 74 made real and `examples/net` proved: connect, listen,
//! accept, read, write, shutdown, and both addresses. Everything else in `sys::net`'s
//! internal contract — timeouts, `set_nonblocking`, `peek`, `try_clone`, `linger`,
//! `nodelay`, `keepalive`, TTL — answers `Unsupported` individually, each for the
//! reason in [`super::no_option`] or beside it. `std` is built to accept that.

use super::addr::InetAddr;
use super::socket::{Socket, shutdown_how};
use super::{no_option, no_timeout};
use crate::fmt;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut};
use crate::net::{Shutdown, SocketAddr, ToSocketAddrs};
use crate::time::Duration;

pub struct TcpStream {
    socket: Socket,
}

impl TcpStream {
    pub(super) fn of_socket(socket: Socket) -> Self {
        TcpStream { socket }
    }

    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
        super::super::each_addr(addr, |addr| {
            let socket = Socket::tcp()?;
            socket.connect(&mut InetAddr::of_socket(addr)?)?;
            Ok(TcpStream { socket })
        })
    }

    /// `RSocket::Connect` takes no deadline and cancelling it would need a second
    /// outstanding request on this thread, which is the one thing
    /// [`super::request::blocking`] must not have. See [`no_timeout`].
    pub fn connect_timeout(_: &SocketAddr, _: Duration) -> io::Result<TcpStream> {
        Err(no_timeout())
    }

    pub fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Err(no_timeout())
    }

    pub fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Err(no_timeout())
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(None)
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(None)
    }

    /// `RSocket::Recv` takes a flags word and `es_sock.h` names `KSockReadPeek`, but no
    /// experiment here has ever issued it, so peeking is refused rather than aimed at
    /// a constant nobody has watched work.
    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        Err(no_option("peek", "RSocket::Recv's KSockReadPeek flag has not been observed"))
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.recv(buf)
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        crate::io::default_read_buf(|buf| self.read(buf), cursor)
    }

    /// One descriptor per call: `RSocket::RecvOneOrMore` takes a single `TDes8&` and
    /// there is no scatter form, so this reads into the first non-empty slice.
    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        match bufs.iter_mut().find(|b| !b.is_empty()) {
            Some(buf) => self.read(buf),
            None => Ok(0),
        }
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.socket.send(buf)
    }

    /// As [`Self::read_vectored`]: `RSocket::Send` takes one `TDesC8&`.
    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        match bufs.iter().find(|b| !b.is_empty()) {
            Some(buf) => self.write(buf),
            None => Ok(0),
        }
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.socket.peer_addr().to_socket()
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr().to_socket()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(shutdown_how(how))
    }

    pub fn duplicate(&self) -> io::Result<TcpStream> {
        Err(no_dup())
    }

    pub fn set_linger(&self, _: Option<Duration>) -> io::Result<()> {
        Err(no_option("SO_LINGER", "no RSocket::SetOpt option has been observed"))
    }

    pub fn linger(&self) -> io::Result<Option<Duration>> {
        Err(no_option("SO_LINGER", "no RSocket::GetOpt option has been observed"))
    }

    pub fn set_keepalive(&self, _: bool) -> io::Result<()> {
        Err(no_option("SO_KEEPALIVE", "no RSocket::SetOpt option has been observed"))
    }

    pub fn keepalive(&self) -> io::Result<bool> {
        Err(no_option("SO_KEEPALIVE", "no RSocket::GetOpt option has been observed"))
    }

    pub fn set_nodelay(&self, _: bool) -> io::Result<()> {
        Err(no_option("TCP_NODELAY", "no RSocket::SetOpt option has been observed"))
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        Err(no_option("TCP_NODELAY", "no RSocket::GetOpt option has been observed"))
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        Err(no_option("IP_TTL", "no RSocket::SetOpt option has been observed"))
    }

    pub fn ttl(&self) -> io::Result<u32> {
        Err(no_option("IP_TTL", "no RSocket::GetOpt option has been observed"))
    }

    /// There is no pending-error queue to drain: every Symbian socket call reports its
    /// own failure, as a `TInt` or through the request status, at the call.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        Ok(None)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        if nonblocking { Err(super::no_nonblocking()) } else { Ok(()) }
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TcpStream").field("peer", &self.peer_addr().ok()).finish()
    }
}

pub struct TcpListener {
    socket: Socket,
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
        super::super::each_addr(addr, |addr| {
            let socket = Socket::tcp()?;
            socket.bind(&mut InetAddr::of_socket(addr)?)?;
            // `KListenQueueSize` is not in the public headers; five is what
            // `examples/net` has listened with since step 74.
            socket.listen(5)?;
            Ok(TcpListener { socket })
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr().to_socket()
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let accepted = self.socket.accept()?;
        let peer = accepted.peer_addr().to_socket()?;
        Ok((TcpStream::of_socket(accepted), peer))
    }

    pub fn duplicate(&self) -> io::Result<TcpListener> {
        Err(no_dup())
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        Err(no_option("IP_TTL", "no RSocket::SetOpt option has been observed"))
    }

    pub fn ttl(&self) -> io::Result<u32> {
        Err(no_option("IP_TTL", "no RSocket::GetOpt option has been observed"))
    }

    /// The socket is opened with `KAfInet`, so there is no v6 half to turn off.
    pub fn set_only_v6(&self, _: bool) -> io::Result<()> {
        Err(super::addr::ipv6_unsupported())
    }

    pub fn only_v6(&self) -> io::Result<bool> {
        Err(super::addr::ipv6_unsupported())
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        Ok(None)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        if nonblocking { Err(super::no_nonblocking()) } else { Ok(()) }
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TcpListener").field("local", &self.socket_addr().ok()).finish()
    }
}

/// `RSocket::Transfer` moves a socket to **another thread**, and `RHandleBase::Duplicate`
/// duplicates a thread- or process-owned handle, not a sub-session. Neither is a dup
/// within one thread, so there is nothing honest to call.
fn no_dup() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "a Symbian socket sub-session cannot be duplicated within a thread: \
         RSocket::Transfer moves one between threads and has not been observed",
    )
}
