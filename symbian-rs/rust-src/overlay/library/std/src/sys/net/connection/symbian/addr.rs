//! `TInetAddr` and the two conversions `std::net::SocketAddr` needs.
//!
//! # IPv6 is `Unsupported`, and that is a statement about this repository
//!
//! `TInetAddr` carries IPv6 and the stack often *answers* in `KAfInet6` with the
//! address in v4-mapped form (`in_sock.h` lines 386-391), which [`InetAddr::to_socket`]
//! converts. But nothing here has ever sent a packet to a real IPv6 address on this
//! stack or on the emulator's, and the scope-id and flow-label fields of
//! `TInetAddr::SetAddress(const TIp6Addr&)` have never been observed, so there is no
//! constructor for one. `TODO: IPv6 (not observed)` — `Unsupported` says so honestly.

use crate::io;
use crate::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use symbian_sys::esock::{
    KAF_INET, KAF_INET6, TInetAddr_Address, TInetAddr_ConvertToV4, TInetAddr_IsV4Mapped,
    TInetAddr_ctor, TInetAddr_default_ctor, TSockAddr, TSockAddrStorage, TSockAddr_Family,
    TSockAddr_Port,
};

/// A `TInetAddr`, built by insock's own exported constructor.
///
/// The 8-bit descriptor header inside a `TSockAddr` was never observed here, so nothing
/// in this type writes one: zeroed storage of the measured `sizeof(TInetAddr)` (40) is
/// handed to insock and insock builds the object.
pub struct InetAddr {
    storage: TSockAddrStorage,
}

impl InetAddr {
    /// An IPv4 address and port. The address is in host order, as
    /// `TInetAddr(TUint32, TUint)` wants it: `127.0.0.1` is `0x7f00_0001`, which is
    /// exactly `u32::from(Ipv4Addr)`.
    pub fn v4(address: u32, port: u16) -> Self {
        let mut storage = TSockAddrStorage::zeroed();
        // SAFETY: zeroed storage of the measured `sizeof(TInetAddr)` (40), 4-aligned as
        // measured, is a valid object to construct into, and `this` is argument 0 under
        // the observed member ABI. `in_sock.h` declares no leaving member, and the
        // constructor only writes its own bytes: it allocates nothing and keeps no
        // pointer.
        unsafe { TInetAddr_ctor(&mut storage, address, u32::from(port)) };
        Self { storage }
    }

    /// An unspecified address for a call that will fill it in.
    ///
    /// It must be **constructed**, not merely zeroed, and step 74 learned that the
    /// expensive way: `TSockAddr` is a `TBuf8<KMaxSockAddrSize>`, so `LocalName`,
    /// `RemoteName` and `RecvFrom` write into it *as a descriptor* and honour its
    /// `iMaxLength`. Zeroed storage has `iMaxLength == 0`, the socket server writes
    /// nothing, and the caller reads back `KAFUnspec` with no diagnostic anywhere.
    pub fn blank() -> Self {
        let mut storage = TSockAddrStorage::zeroed();
        // SAFETY: as [`Self::v4`], with the no-argument constructor.
        unsafe { TInetAddr_default_ctor(&mut storage) };
        Self { storage }
    }

    /// A copy of the `TInetAddr` a resolver answer holds.
    ///
    /// # Safety
    ///
    /// `answer` must point at a live, constructed `TSockAddr` of the measured 40 bytes
    /// — which is what `TNameEntryStorage::addr_mut` returns after `RHostResolver` has
    /// written into it.
    pub unsafe fn of_answer(answer: *mut TSockAddrStorage) -> Self {
        // SAFETY: the caller's contract. `TSockAddrStorage` is plain bytes with no
        // interior pointer, so a byte copy of it is another valid `TSockAddr`.
        Self { storage: unsafe { crate::ptr::read(answer) } }
    }

    /// What `std` hands the platform layer. IPv6 is refused rather than guessed at.
    pub fn of_socket(addr: &SocketAddr) -> io::Result<Self> {
        match addr {
            SocketAddr::V4(v4) => Ok(Self::v4(u32::from(*v4.ip()), v4.port())),
            SocketAddr::V6(_) => Err(ipv6_unsupported()),
        }
    }

    /// The address as `std` spells it, converting a v4-mapped `KAfInet6` answer the way
    /// `in_sock.h` says a caller must expect.
    pub fn to_socket(&mut self) -> io::Result<SocketAddr> {
        match self.family() {
            KAF_INET => {}
            // SAFETY: a `const` member and then a non-`const` one, both non-leaving,
            // both taking only `this`; the storage is a live `TInetAddr` the stack or
            // the resolver wrote.
            KAF_INET6 if unsafe { TInetAddr_IsV4Mapped(&self.storage) } != 0 => unsafe {
                TInetAddr_ConvertToV4(&mut self.storage);
            },
            _ => return Err(ipv6_unsupported()),
        }
        // SAFETY: as above; the family is now `KAfInet`, so `Address()` is meaningful.
        let raw = unsafe { TInetAddr_Address(&self.storage) };
        Ok(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(raw), self.port())))
    }

    /// The address family (`TSockAddr::Family`).
    fn family(&self) -> u32 {
        // SAFETY: a `const` member with no argument but `this`, non-leaving; the
        // storage is a live `TSockAddr` (or zeroed, which reads as `KAFUnspec`).
        unsafe { TSockAddr_Family(&self.storage) }
    }

    /// The port. Symbian's accessor is a `TUint`; a port that does not fit 16 bits
    /// cannot come from an IP stack, so this truncates rather than inventing an error
    /// for a case the protocol makes impossible.
    fn port(&self) -> u16 {
        // SAFETY: as [`Self::family`].
        (unsafe { TSockAddr_Port(&self.storage) }) as u16
    }

    /// The `TSockAddr&` a socket call takes. Every one of them — `Connect`, `Bind`,
    /// `SendTo` — takes it non-`const` even where it only reads it.
    pub fn as_sockaddr_mut(&mut self) -> *mut TSockAddr {
        self.storage.as_sockaddr_mut()
    }
}

/// The one error every IPv6 path in this backend returns.
pub fn ipv6_unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "TODO: IPv6 (not observed) — TInetAddr carries it, but no experiment here has \
         ever sent a packet to an IPv6 address on this stack",
    )
}
