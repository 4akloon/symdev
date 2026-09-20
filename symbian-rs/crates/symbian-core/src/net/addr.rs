//! `InetAddr`: a `TInetAddr`, the address every socket call takes.
use symbian_sys::esock::{
    KAF_INET, KAF_INET6, TInetAddr_Address, TInetAddr_ConvertToV4, TInetAddr_IsV4Mapped,
    TInetAddr_ctor, TSockAddr, TSockAddr_Family, TSockAddr_Port, TSockAddrStorage,
};

use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// An IPv4 endpoint, built by insock's own exported constructor.
///
/// The descriptor header word inside a `TSockAddr` was never observed here (the 8-bit
/// type nibbles are the open question of experiment 79), so this type never writes one:
/// it hands zeroed storage of the measured size to `TInetAddr::TInetAddr(TUint32, TUint)`
/// and lets insock build the object, exactly as [`crate::des8`] does for `TPtrC8`.
///
/// # IPv6 is not here
///
/// `TInetAddr` carries IPv6 too, and the stack often *answers* in `KAfInet6` with an
/// IPv4 address in v4-mapped form — [`InetAddr::to_v4`] is how such an answer is brought
/// back. But nothing in this repository has ever exercised a real IPv6 address through
/// `SetAddress(const TIp6Addr&)`, so there is no constructor for one; see
/// [`InetAddr::v6_unsupported`].
pub struct InetAddr {
    storage: TSockAddrStorage,
}

impl InetAddr {
    /// An IPv4 address and port (`TInetAddr(TUint32 aAddr, TUint aPort)`), with the
    /// address in host order — `127.0.0.1` is `0x7f00_0001`.
    pub fn v4(address: u32, port: u16) -> Self {
        let mut storage = TSockAddrStorage::zeroed();
        // SAFETY: `storage` is zeroed storage of the measured `sizeof(TInetAddr)` (40),
        // 4-aligned as measured, so it is a valid object to construct into; `this` is
        // argument 0 under the observed member ABI. `in_sock.h` declares no leaving
        // member at all, and a `TInetAddr` constructor only writes its own bytes: it
        // allocates nothing and keeps no pointer.
        unsafe { TInetAddr_ctor(&mut storage, address, u32::from(port)) };
        Self { storage }
    }

    /// `0.0.0.0` on `port`: what a listener binds to when it does not name an interface.
    pub fn any(port: u16) -> Self {
        Self::v4(0, port)
    }

    /// Zeroed storage that a call such as `RSocket::RemoteName` will fill in.
    ///
    /// It is not a valid address until something writes one: reading
    /// [`InetAddr::family`] from it gives `0` (`KAFUnspec`).
    pub(crate) const fn blank() -> Self {
        Self {
            storage: TSockAddrStorage::zeroed(),
        }
    }

    /// The address family (`TSockAddr::Family`): `KAfInet`, `KAfInet6` or `KAFUnspec`.
    pub fn family(&self) -> u32 {
        // SAFETY: a `const` member with no argument but `this`, non-leaving; the
        // storage is a live `TSockAddr` (or zeroed, which reads as `KAFUnspec`).
        unsafe { TSockAddr_Family(&self.storage) }
    }

    /// The port (`TSockAddr::Port`).
    ///
    /// Symbian's accessor is a `TUint`; a port that does not fit 16 bits cannot come
    /// from an IP stack, and this truncates rather than inventing an error for a case
    /// the protocol makes impossible.
    pub fn port(&self) -> u16 {
        // SAFETY: as [`Self::family`].
        (unsafe { TSockAddr_Port(&self.storage) }) as u16
    }

    /// The IPv4 address in host order (`TInetAddr::Address`), or
    /// [`ErrorKind::NotSupported`] for an address that is not IPv4.
    ///
    /// An answer the stack returned in `KAfInet6` v4-mapped form is converted first, as
    /// `in_sock.h` lines 386-391 say a caller must expect; a genuine IPv6 address is the
    /// unsupported case.
    pub fn to_v4(&mut self) -> Result<u32> {
        match self.family() {
            KAF_INET => {}
            // SAFETY: a `const` member and then a non-`const` one, both non-leaving,
            // both taking only `this`; the storage is a live `TInetAddr` that the
            // resolver or the stack wrote.
            KAF_INET6 if unsafe { TInetAddr_IsV4Mapped(&self.storage) } != 0 => unsafe {
                TInetAddr_ConvertToV4(&mut self.storage);
            },
            _ => return Err(Self::v6_unsupported()),
        }
        // SAFETY: as above; the family is now `KAfInet`, so `Address()` is meaningful.
        Ok(unsafe { TInetAddr_Address(&self.storage) })
    }

    /// The error every IPv6 path in this crate returns.
    ///
    /// `TODO: IPv6 (not observed)` — `TInetAddr::SetAddress(const TIp6Addr&)` and
    /// `Ip6Address()` exist and are declared non-leaving, but no experiment here has
    /// ever sent a packet to an IPv6 address on this stack or on the emulator's, and a
    /// guess about the scope-id and flow-label fields is not acceptable (design spec
    /// §7). `KErrNotSupported` says so honestly instead.
    pub const fn v6_unsupported() -> SymbianError {
        SymbianError::of(ErrorKind::NotSupported)
    }

    /// The `TSockAddr&` a socket call takes. Every one of them — `Connect`, `Bind`,
    /// `SendTo` — takes it non-`const` even where it only reads it.
    pub(crate) const fn as_sockaddr_mut(&mut self) -> *mut TSockAddr {
        self.storage.as_sockaddr_mut()
    }
}
