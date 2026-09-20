//! `net::TcpStream`: `std`'s shape over a connected `RSocket`.
use symbian_core::net::{Socket, with_session};

use super::{Shutdown, SocketAddr, ToSocketAddrs, from_inet, to_inet, to_socket_addrs};
use crate::io::{Read, Result, Write};

/// A TCP connection, as `std::net::TcpStream`.
///
/// The socket closes when the value is dropped. Where `std` takes `&self`, this takes
/// `&mut self`: `RSocket::Send` and `RSocket::RecvOneOrMore` are non-`const` members of
/// the handle, and the borrow checker says so rather than hiding it.
pub struct TcpStream {
    socket: Socket,
}

impl TcpStream {
    pub(crate) const fn of(socket: Socket) -> Self {
        Self { socket }
    }

    /// Opens a TCP connection to `addr` and blocks until the stack answers
    /// (`RSocket::Open` then `RSocket::Connect` + `User::WaitForRequest`).
    ///
    /// A `&str` host is resolved first, in its own socket-server borrow, because the
    /// resolver is a sub-session of the same session the socket will be.
    ///
    /// On a device this is also where a missing access point shows up: with no default
    /// connection configured, the socket server has nowhere to send the packets. There
    /// is no `std` word for that and this call does not invent one — the Symbian code
    /// arrives through [`crate::io::Error::raw_os_error`].
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = to_socket_addrs::first(&addr)?;
        let mut target = to_inet(addr)?;
        let mut socket = with_session(Socket::tcp)?;
        socket.connect(&mut target)?;
        Ok(Self::of(socket))
    }

    /// The peer's address (`RSocket::RemoteName`).
    pub fn peer_addr(&mut self) -> Result<SocketAddr> {
        from_inet(self.socket.peer_addr())
    }

    /// This end's address (`RSocket::LocalName`).
    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        from_inet(self.socket.local_addr())
    }

    /// Closes one or both halves of the connection (`RSocket::Shutdown`), blocking
    /// until the stack has done it.
    ///
    /// `Shutdown::Write` is what a protocol that ends with "the client stops talking"
    /// needs: it sends the FIN and leaves the read half open for the answer.
    pub fn shutdown(&mut self, how: Shutdown) -> Result<()> {
        Ok(self.socket.shutdown(how.to_symbian())?)
    }
}

impl Read for TcpStream {
    /// `RSocket::RecvOneOrMore`, which completes as soon as any bytes have arrived —
    /// `RSocket::Recv` would wait for the buffer to be full, which is not what
    /// `std::io::Read` promises.
    ///
    /// `Ok(0)` is the peer having closed its end, as in `std`. Underneath, Symbian says
    /// that with `KErrEof` (-25) rather than with a zero-byte completion — observed in
    /// the emulator, where `read_to_end` first came back as
    /// `UnexpectedEof (KErrEof (-25))` — and `symbian_core::net::Socket::recv` reports
    /// it as the bytes that arrived, which is the same translation `RFile::Read` makes
    /// for end of file.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.socket.recv(buf)?)
    }
}

impl Write for TcpStream {
    /// `RSocket::Send` sends the whole descriptor or fails, so this never reports a
    /// short write and [`Write::write_all`] never has to loop — Symbian being stricter
    /// than `std`, which is the safe direction.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.socket.send(buf)?;
        Ok(buf.len())
    }

    /// Nothing to do: `RSocket::Send` has already blocked until the stack took the
    /// bytes, so there is no buffer of ours left to push.
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
