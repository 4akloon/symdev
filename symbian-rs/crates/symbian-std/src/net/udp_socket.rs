//! `net::UdpSocket`: `std`'s shape over a datagram `RSocket`.
use symbian_core::net::{Socket, with_session};

use super::{SocketAddr, ToSocketAddrs, from_inet, to_inet, to_socket_addrs};
use crate::io::Result;

/// A UDP socket, as `std::net::UdpSocket`.
///
/// It came cheaply: the same `RSocket` with `KSockDatagram`/`KProtocolInetUdp`, and
/// `SendTo`/`RecvFrom` in place of `Send`/`RecvOneOrMore`. Where `std` takes `&self`,
/// this takes `&mut self`, for the reason on [`super::TcpStream`].
///
/// `connect`, `send`, `recv`, `peek_from`, `set_broadcast` and the multicast setters are
/// not here. Each is a real Symbian call and none has been run, so none is shipped.
pub struct UdpSocket {
    socket: Socket,
}

impl UdpSocket {
    /// Binds a UDP socket to `addr` (`RSocket::Open` then `Bind`). Port 0 asks the
    /// stack to choose; [`UdpSocket::local_addr`] then says which.
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = to_socket_addrs::first(&addr)?;
        let mut local = to_inet(addr)?;
        let mut socket = with_session(Socket::udp)?;
        socket.bind(&mut local)?;
        Ok(Self { socket })
    }

    /// Sends one datagram (`RSocket::SendTo`) and blocks until the stack has taken it.
    ///
    /// Returns `buf.len()`: Symbian sends the whole descriptor or fails, so a partial
    /// datagram is not a case this can report.
    pub fn send_to(&mut self, buf: &[u8], addr: impl ToSocketAddrs) -> Result<usize> {
        let addr = to_socket_addrs::first(&addr)?;
        let mut target = to_inet(addr)?;
        self.socket.send_to(buf, &mut target)?;
        Ok(buf.len())
    }

    /// Blocks for one datagram and says where it came from (`RSocket::RecvFrom`).
    ///
    /// A datagram longer than `buf` is truncated, as in `std` on a POSIX system; there
    /// is no flag here that would report the loss, so a caller that cares about it
    /// should offer a buffer of the protocol's maximum.
    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (read, from) = self.socket.recv_from(buf)?;
        Ok((read, from_inet(from)?))
    }

    /// The address this socket is bound to (`RSocket::LocalName`).
    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        from_inet(self.socket.local_addr())
    }
}
