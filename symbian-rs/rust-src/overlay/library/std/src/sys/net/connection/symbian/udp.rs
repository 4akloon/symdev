//! `UdpSocket` over `RSocket` with `KSockDatagram` and `KProtocolInetUdp`.
//!
//! Real: `bind`, `send_to`, `recv_from`, `local_addr`, and `connect`/`send`/`recv`
//! against a remembered peer. Every socket option — broadcast, TTL, multicast — is
//! `Unsupported`: they are all `RSocket::SetOpt`/`GetOpt` with an option constant, and
//! no experiment in this repository has ever set one and watched what changed.

use super::addr::{InetAddr, ipv6_unsupported};
use super::socket::Socket;
use super::{no_option, no_timeout};
use crate::cell::RefCell;
use crate::fmt;
use crate::io;
use crate::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use crate::time::Duration;

pub struct UdpSocket {
    socket: Socket,
    /// The peer `connect` remembered. Symbian's datagram socket has a `Connect` of its
    /// own, but nothing here has observed what the stack does with it for UDP, so the
    /// peer is kept on this side and `send`/`recv` become `SendTo`/`RecvFrom`, which
    /// are observed.
    peer: RefCell<Option<SocketAddr>>,
}

// SAFETY: as `Socket`'s own impl, and for the same reason: `std::net::UdpSocket` is
// documented `Send + Sync`. The `RefCell` is this type's own bookkeeping and is never
// held across a call into the socket server.
unsafe impl Sync for UdpSocket {}

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
        super::super::each_addr(addr, |addr| {
            let socket = Socket::udp()?;
            socket.bind(&mut InetAddr::of_socket(addr)?)?;
            Ok(UdpSocket { socket, peer: RefCell::new(None) })
        })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.peer.borrow().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "this datagram socket has no peer")
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr().to_socket()
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (read, mut from) = self.socket.recv_from(buf)?;
        Ok((read, from.to_socket()?))
    }

    pub fn peek_from(&self, _: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        Err(no_option("peek_from", "RSocket::RecvFrom's KSockReadPeek flag has not been observed"))
    }

    pub fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> io::Result<usize> {
        self.socket.send_to(buf, &mut InetAddr::of_socket(addr)?)
    }

    pub fn duplicate(&self) -> io::Result<UdpSocket> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "a Symbian socket sub-session cannot be duplicated within a thread",
        ))
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

    pub fn set_broadcast(&self, _: bool) -> io::Result<()> {
        Err(no_option("SO_BROADCAST", "no RSocket::SetOpt option has been observed"))
    }

    pub fn broadcast(&self) -> io::Result<bool> {
        Err(no_option("SO_BROADCAST", "no RSocket::GetOpt option has been observed"))
    }

    pub fn set_multicast_loop_v4(&self, _: bool) -> io::Result<()> {
        Err(no_multicast())
    }

    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        Err(no_multicast())
    }

    pub fn set_multicast_ttl_v4(&self, _: u32) -> io::Result<()> {
        Err(no_multicast())
    }

    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        Err(no_multicast())
    }

    pub fn set_multicast_loop_v6(&self, _: bool) -> io::Result<()> {
        Err(ipv6_unsupported())
    }

    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        Err(ipv6_unsupported())
    }

    pub fn join_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        Err(no_multicast())
    }

    pub fn join_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        Err(ipv6_unsupported())
    }

    pub fn leave_multicast_v4(&self, _: &Ipv4Addr, _: &Ipv4Addr) -> io::Result<()> {
        Err(no_multicast())
    }

    pub fn leave_multicast_v6(&self, _: &Ipv6Addr, _: u32) -> io::Result<()> {
        Err(ipv6_unsupported())
    }

    pub fn set_ttl(&self, _: u32) -> io::Result<()> {
        Err(no_option("IP_TTL", "no RSocket::SetOpt option has been observed"))
    }

    pub fn ttl(&self) -> io::Result<u32> {
        Err(no_option("IP_TTL", "no RSocket::GetOpt option has been observed"))
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        Ok(None)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        if nonblocking { Err(super::no_nonblocking()) } else { Ok(()) }
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let peer = self.peer_addr()?;
        loop {
            let (read, mut from) = self.socket.recv_from(buf)?;
            // `connect` on a datagram socket means "only this peer", and the filtering
            // is done here because the stack was never watched doing it.
            if from.to_socket()? == peer {
                return Ok(read);
            }
        }
    }

    pub fn peek(&self, _: &mut [u8]) -> io::Result<usize> {
        Err(no_option("peek", "RSocket::RecvFrom's KSockReadPeek flag has not been observed"))
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let peer = self.peer_addr()?;
        self.send_to(buf, &peer)
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let addr = addr.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "no address to connect the socket to")
        })?;
        // Refuse an IPv6 peer here rather than at the first send.
        InetAddr::of_socket(&addr)?;
        *self.peer.borrow_mut() = Some(addr);
        Ok(())
    }
}

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpSocket").field("local", &self.socket_addr().ok()).finish()
    }
}

/// Multicast is `RSocket::SetOpt` with `KSoIp6JoinGroup`/`KSoIpMulticast*` and a
/// packaged `TIp6Mreq`. Neither the option constants nor the packed request has been
/// observed on this stack, so joining a group is refused rather than guessed at.
fn no_multicast() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "TODO: IP multicast (not observed) — it is RSocket::SetOpt with an option \
         constant and a packaged request, and nothing here has watched one work",
    )
}
