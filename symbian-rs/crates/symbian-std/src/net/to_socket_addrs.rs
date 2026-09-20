//! `ToSocketAddrs`: `std`'s conversion, with `RHostResolver` behind the `&str` cases.
use alloc::vec;
use alloc::vec::{IntoIter, Vec};

use symbian_core::net::{HostResolver, with_session};

use super::{SocketAddr, SocketAddrV4, SocketAddrV6};
use crate::io::{Error, ErrorKind, Result};

/// A value that names one or more socket addresses, as `std::net::ToSocketAddrs`.
///
/// The `&str` implementations resolve through `RHostResolver`, which needs the socket
/// server — so calling one from inside another socket operation is `KErrInUse`. Nothing
/// in this module does; each entry point resolves first and opens afterwards.
pub trait ToSocketAddrs {
    /// The iterator `std` names the same way.
    type Iter: Iterator<Item = SocketAddr>;

    /// The addresses this value names.
    fn to_socket_addrs(&self) -> Result<Self::Iter>;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        Ok(vec![*self].into_iter())
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        SocketAddr::V4(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        SocketAddr::V6(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for (core::net::IpAddr, u16) {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        SocketAddr::new(self.0, self.1).to_socket_addrs()
    }
}

impl ToSocketAddrs for (core::net::Ipv4Addr, u16) {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        SocketAddr::V4(SocketAddrV4::new(self.0, self.1)).to_socket_addrs()
    }
}

impl ToSocketAddrs for [SocketAddr] {
    type Iter = IntoIter<SocketAddr>;

    /// The addresses are copied into a `Vec` rather than borrowed, because [`Self::Iter`]
    /// carries no lifetime. `std` gives its slice implementation a borrowing iterator by
    /// implementing the trait for `&'a [SocketAddr]`; that shape overlaps the blanket
    /// `&T` implementation below, and a `Vec` of socket addresses is cheap.
    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        Ok(Vec::from(self).into_iter())
    }
}

/// `("host", port)`: a literal IPv4 address if it parses as one, otherwise a name for
/// `RHostResolver::GetByName`.
impl ToSocketAddrs for (&str, u16) {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        let (host, port) = *self;
        if let Ok(ip) = host.parse::<core::net::Ipv4Addr>() {
            return Ok(vec![SocketAddr::V4(SocketAddrV4::new(ip, port))].into_iter());
        }
        Ok(vec![resolve(host, port)?].into_iter())
    }
}

/// `"host:port"`, as `std` accepts it.
///
/// The port is what follows the **last** colon, so an IPv6 literal in brackets is
/// rejected by the address parser rather than mis-split — and it would be
/// [`ErrorKind::Unsupported`] anyway.
impl ToSocketAddrs for str {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        let Some((host, port)) = self.rsplit_once(':') else {
            return Err(Error::from(ErrorKind::InvalidInput));
        };
        let Ok(port) = port.parse::<u16>() else {
            return Err(Error::from(ErrorKind::InvalidInput));
        };
        (host, port).to_socket_addrs()
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        (**self).to_socket_addrs()
    }
}

/// The first address `host` resolves to (`RHostResolver::GetByName`).
///
/// `TODO: more than one answer (not observed)` — `RHostResolver::Next` walks the rest,
/// and `symbian_core::net::HostResolver::next` is there for it, but no run here has ever
/// produced a second answer, so this does not pretend to iterate.
fn resolve(host: &str, port: u16) -> Result<SocketAddr> {
    let addr = with_session(|server| HostResolver::open(server)?.lookup(host, port))?;
    super::from_inet(addr)
}

/// The one address `addr` names, or [`ErrorKind::InvalidInput`] for a value that names
/// none — which is what `std` reports for an empty slice.
pub(crate) fn first(addr: &impl ToSocketAddrs) -> Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| Error::from(ErrorKind::InvalidInput))
}
