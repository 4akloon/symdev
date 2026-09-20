//! `net::TcpListener`: `std`'s shape over a listening `RSocket`.
use symbian_core::net::{Socket, with_session};

use super::{SocketAddr, TcpStream, ToSocketAddrs, from_inet, to_inet, to_socket_addrs};
use crate::io::Result;

/// How many pending connections the socket server keeps. `std` does not let a caller
/// name this either (`TcpListener::bind` hard-codes 128 on Unix); `RSocket::Listen`
/// demands a number, so this is it.
const BACKLOG: u32 = 8;

/// A TCP server socket, as `std::net::TcpListener`.
///
/// Where `std` takes `&self`, this takes `&mut self`: `RSocket::Accept` is a non-`const`
/// member of the handle, and accepting also needs the socket-server session, because the
/// connection arrives in a **blank** socket that has to be opened on the same session
/// first (`RSocket::Open(RSocketServ&)`).
pub struct TcpListener {
    socket: Socket,
}

impl TcpListener {
    /// Binds to `addr` and starts listening (`RSocket::Open`, `Bind`, `Listen`).
    ///
    /// Port 0 asks the stack to choose; [`TcpListener::local_addr`] then says which.
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = to_socket_addrs::first(&addr)?;
        let mut local = to_inet(addr)?;
        let mut socket = with_session(Socket::tcp)?;
        socket.bind(&mut local)?;
        socket.listen(BACKLOG)?;
        Ok(Self { socket })
    }

    /// Blocks for one connection (`RSocket::Accept` + `User::WaitForRequest`).
    pub fn accept(&mut self) -> Result<(TcpStream, SocketAddr)> {
        let mut blank = with_session(Socket::blank)?;
        self.socket.accept(&mut blank)?;
        let peer = from_inet(blank.peer_addr())?;
        Ok((TcpStream::of(blank), peer))
    }

    /// The address this listener is bound to (`RSocket::LocalName`), which is how a
    /// listener bound to port 0 learns the port it got.
    pub fn local_addr(&mut self) -> Result<SocketAddr> {
        from_inet(self.socket.local_addr())
    }

    /// An endless iterator over incoming connections, as `std::net::TcpListener::incoming`.
    ///
    /// It borrows mutably, where `std`'s borrows shared, for the reason on the type.
    pub fn incoming(&mut self) -> Incoming<'_> {
        Incoming { listener: self }
    }
}

/// The iterator [`TcpListener::incoming`] returns. It never ends: a failed accept is an
/// `Err` item, exactly as in `std`.
pub struct Incoming<'a> {
    listener: &'a mut TcpListener,
}

impl Iterator for Incoming<'_> {
    type Item = Result<TcpStream>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.listener.accept().map(|(stream, _)| stream))
    }
}
