//! `TSockAddr` (`es_sock.h`, esock.dso) and `TInetAddr` (`in_sock.h`, insock.dso): the
//! end-point address every socket call takes by reference.
//!
//! **Nothing about the layout is assumed.** `TSockAddr` is
//! `class TSockAddr : public TBuf8<KMaxSockAddrSize>` with `KMaxSockAddrSize = 0x20`
//! (`es_sock.h` lines 193 and 195), so it begins with a descriptor header word whose type
//! nibble was never observed here — exactly the situation of the 8-bit descriptors in
//! [`crate::des8`], and answered the same way: the storage is zeroed bytes of the measured
//! size and the *exported constructor* builds the object. `sizeof(TSockAddr)` and
//! `sizeof(TInetAddr)` are both **40**, alignment **4**, measured by compiling
//! `return sizeof(TInetAddr);` with the recorded GCCE argv (`TInetAddr` adds no data
//! members; it reinterprets the `TBuf8` body its base already has).
//!
//! Every member below is a non-static, non-virtual member with scalar arguments, called
//! with `this` as argument 0 under the member ABI observed in experiment 78. `in_sock.h`
//! declares no leaving member at all, so none of it goes through a shim.

/// `KAFUnspec` (`es_sock.h` line 235): no address family. A `TInetAddr` in this family
/// carries no address and is what the stack treats as "unspecified" rather than as an
/// explicit `0.0.0.0`.
pub const KAF_UNSPEC: u32 = 0;

/// `KAfInet` (`in_sock.h` line 41): the IPv4 family, and the only *protocol* family
/// constant `in_sock.h` blesses.
pub const KAF_INET: u32 = 0x0800;

/// `KAfInet6` (`in_sock.h` line 47): the IPv6 *address* family, which also carries IPv4
/// addresses in v4-mapped form. The stack often answers in this family even for an IPv4
/// question, which is why [`TInetAddr_IsV4Mapped`] and [`TInetAddr_ConvertToV4`] are
/// declared here.
pub const KAF_INET6: u32 = 0x0806;

/// The opaque `TSockAddr` an export takes by `&` or `const&`; only ever seen behind a
/// pointer. `TInetAddr` derives from it with single, non-virtual inheritance and adds no
/// members, so the same pointer serves for both and no `this` adjustment arises.
#[repr(C)]
pub struct TSockAddr {
    _private: [u8; 0],
}

/// Storage for a `TSockAddr` or a `TInetAddr`: `sizeof == 40`, `alignof == 4`, measured.
///
/// Zeroed storage is not a valid address — the descriptor header word has to be built —
/// so every path constructs into it with [`TInetAddr_ctor`] before handing it to a
/// socket call.
#[repr(C, align(4))]
pub struct TSockAddrStorage {
    bytes: [u8; 40],
}

impl TSockAddrStorage {
    /// Zeroed storage, ready for insock's constructor to build an address into.
    pub const fn zeroed() -> Self {
        Self { bytes: [0; 40] }
    }

    /// The `const TSockAddr&` a reading export expects.
    pub const fn as_sockaddr(&self) -> *const TSockAddr {
        (self as *const Self).cast()
    }

    /// The `TSockAddr&` a writing export expects.
    ///
    /// `RSocket::Connect`, `Bind` and `SendTo` all take a non-`const` `TSockAddr&` even
    /// where they only read it, so this is needed on the sending path too.
    pub const fn as_sockaddr_mut(&mut self) -> *mut TSockAddr {
        (self as *mut Self).cast()
    }
}

unsafe extern "C" {
    /// `00000030 T _ZN9TInetAddrC1Ev` — `TInetAddr::TInetAddr()`, insock.dso: an
    /// unspecified (`KAFUnspec`) address.
    ///
    /// This is what an address a *call* fills in has to be built with, and the reason is
    /// observed rather than theoretical. `TSockAddr` is a `TBuf8<KMaxSockAddrSize>`, so
    /// `RSocket::LocalName`, `RemoteName` and `RecvFrom` write into it **as a
    /// descriptor** — the socket server checks `iMaxLength` and sets `iLength`. Handed
    /// zeroed storage instead, they write nothing: `examples/net` reported
    /// `peer_addr`, `local_addr` and the UDP port as failures until this constructor was
    /// called first.
    #[link_name = "_ZN9TInetAddrC1Ev"]
    pub fn TInetAddr_default_ctor(this: *mut TSockAddrStorage);

    /// `00000000 T _ZN9TInetAddrC1Emj` — `TInetAddr::TInetAddr(TUint32 aAddr, TUint
    /// aPort)`, insock.dso. Builds a `KAfInet` address in `this` from a host-order IPv4
    /// address and a port, so Rust never writes the descriptor header word.
    ///
    /// `TUint32` mangles as `m` (`unsigned long`) on this ABI, which is what fixes this
    /// overload against `TInetAddr(TUint aPort)` (`_ZN9TInetAddrC1Ej`).
    #[link_name = "_ZN9TInetAddrC1Emj"]
    pub fn TInetAddr_ctor(this: *mut TSockAddrStorage, addr: u32, port: u32);

    /// `0000003c T _ZNK9TInetAddr7AddressEv` — `TUint32 TInetAddr::Address() const`:
    /// the IPv4 address in host order, or `0` when the address is not IPv4.
    #[link_name = "_ZNK9TInetAddr7AddressEv"]
    pub fn TInetAddr_Address(this: *const TSockAddrStorage) -> u32;

    /// `00000024 T _ZN9TInetAddr10SetAddressEm` — `void TInetAddr::SetAddress(TUint32
    /// aAddr)`: replaces the address bits, resetting scope id and flow label.
    #[link_name = "_ZN9TInetAddr10SetAddressEm"]
    pub fn TInetAddr_SetAddress(this: *mut TSockAddrStorage, addr: u32);

    /// `00000094 T _ZNK9TInetAddr10IsV4MappedEv` — `TBool TInetAddr::IsV4Mapped() const`:
    /// whether a `KAfInet6` address is an IPv4 address in v4-mapped form. The resolver
    /// and the stack often answer in `KAfInet6` even to an IPv4 question (`in_sock.h`
    /// lines 386-391), so this is how an answer is recognised as IPv4.
    #[link_name = "_ZNK9TInetAddr10IsV4MappedEv"]
    pub fn TInetAddr_IsV4Mapped(this: *const TSockAddrStorage) -> i32;

    /// `00000048 T _ZN9TInetAddr11ConvertToV4Ev` — `void TInetAddr::ConvertToV4()`:
    /// rewrites a v4-mapped or v4-compatible `KAfInet6` address as a plain `KAfInet` one.
    #[link_name = "_ZN9TInetAddr11ConvertToV4Ev"]
    pub fn TInetAddr_ConvertToV4(this: *mut TSockAddrStorage);

    /// `0000045c T _ZNK9TSockAddr6FamilyEv` — `TUint TSockAddr::Family() const`,
    /// esock.dso.
    #[link_name = "_ZNK9TSockAddr6FamilyEv"]
    pub fn TSockAddr_Family(this: *const TSockAddrStorage) -> u32;

    /// `0000040c T _ZN9TSockAddr9SetFamilyEj` — `void TSockAddr::SetFamily(TUint
    /// aFamily)`, esock.dso.
    #[link_name = "_ZN9TSockAddr9SetFamilyEj"]
    pub fn TSockAddr_SetFamily(this: *mut TSockAddrStorage, family: u32);

    /// `00000458 T _ZNK9TSockAddr4PortEv` — `TUint TSockAddr::Port() const`, esock.dso.
    #[link_name = "_ZNK9TSockAddr4PortEv"]
    pub fn TSockAddr_Port(this: *const TSockAddrStorage) -> u32;

    /// `00000408 T _ZN9TSockAddr7SetPortEj` — `void TSockAddr::SetPort(TUint aPort)`,
    /// esock.dso.
    #[link_name = "_ZN9TSockAddr7SetPortEj"]
    pub fn TSockAddr_SetPort(this: *mut TSockAddrStorage, port: u32);
}
