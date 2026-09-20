//! `HostResolver`: name resolution (`RHostResolver`).
//!
//! `RHostResolver::GetByName` has a **synchronous** overload returning a `TInt`, so
//! resolving a name needs no `TRequestStatus` and no wait at all. Nothing on `RHostResolver`
//! leaves (`es_sock.h` lines 849-906), so there is no C++ in the path either.
use symbian_sys::esock::{
    KAF_INET, KAF_INET6, KPROTOCOL_INET_TCP, RHostResolver, RHostResolver_Close,
    RHostResolver_GetByName, RHostResolver_Next, RHostResolver_Open, TInetAddr_Address,
    TInetAddr_ConvertToV4, TInetAddr_IsV4Mapped, TNAME_RECORD_SIZE, TNameEntryStorage,
    TPckgBuf_ctor, TSockAddr_Family,
};

use super::addr::InetAddr;
use super::server::SocketServer;
use crate::des::{Buf16, DesC16};
use crate::error::{Result, check};
use crate::{ErrorKind, SymbianError};

/// `KMaxHostName`: `THostName` is `TBuf<0x100>` (`es_sock.h` line 553), so a name longer
/// than 256 code units cannot be asked about and is `KErrOverflow` before any call.
pub const MAX_HOST_NAME: usize = 0x100;

/// An open resolver sub-session.
///
/// Not `Send` or `Sync`: a sub-session handle belongs to the thread that opened it.
pub struct HostResolver {
    resolver: RHostResolver,
    answer: TNameEntryStorage,
}

impl HostResolver {
    /// Opens a resolver for IPv4 over TCP (`RHostResolver::Open(server, KAfInet,
    /// KProtocolInetTcp)`).
    ///
    /// The protocol argument picks which protocol module answers, not what the answer
    /// is about: a TCP resolver resolves the same names a UDP one does.
    pub fn open(server: &mut SocketServer) -> Result<Self> {
        let mut resolver = Self {
            resolver: RHostResolver {
                session_handle: 0,
                sub_session_handle: 0,
            },
            answer: TNameEntryStorage::zeroed(),
        };
        // SAFETY: zeroed storage of the measured `sizeof(TNameEntry)` (576), 8-aligned
        // as `TAlignedBuf8` requires, handed to euser's own exported
        // `TBufBase8(TInt,TInt)` with `sizeof(TNameRecord)` (564, measured) for both —
        // which is what the inline `TPckgBuf<TNameRecord>()` does (`e32cmn.inl` lines
        // 2659 and 1179). The header word is built by euser and never guessed here.
        unsafe {
            TPckgBuf_ctor(
                resolver.answer.as_bytes_mut(),
                TNAME_RECORD_SIZE,
                TNAME_RECORD_SIZE,
            );
        }
        // SAFETY: `this` in argument 0 per the observed member ABI, then the session
        // (borrowed mutably, as `RSocketServ&` is) and two scalars. Non-leaving; on
        // failure the handles stay zero and `Drop` is safe on those.
        let code = unsafe {
            RHostResolver_Open(
                &mut resolver.resolver,
                server.as_server(),
                KAF_INET,
                KPROTOCOL_INET_TCP,
            )
        };
        check(code)?;
        Ok(resolver)
    }

    /// The first IPv4 address `name` resolves to, on `port`
    /// (`RHostResolver::GetByName`).
    ///
    /// `KErrNotFound` for a name that does not resolve; [`ErrorKind::Overflow`] for one
    /// longer than [`MAX_HOST_NAME`], raised here rather than by overflowing a
    /// descriptor, because a descriptor overflow is a panic no `TRAP` catches.
    pub fn lookup(&mut self, name: &str, port: u16) -> Result<InetAddr> {
        let mut buf = Buf16::<MAX_HOST_NAME>::new();
        buf.push_str(name)?;
        // SAFETY: `this` in argument 0; the name is a live `TBuf16` borrowed and only
        // read, and `answer` is the `TPckgBuf<TNameRecord>` built above, borrowed
        // mutably. This is the synchronous overload, so it returns the `TInt` itself
        // and nothing outlives the call. Non-leaving.
        let code = unsafe {
            RHostResolver_GetByName(
                &mut self.resolver,
                buf.as_tdesc16(),
                self.answer.as_name_entry(),
            )
        };
        check(code)?;
        self.answer_as_v4(port)
    }

    /// The next address for the same question, or `KErrEof` when there are no more
    /// (`RHostResolver::Next`).
    pub fn next(&mut self, port: u16) -> Result<InetAddr> {
        // SAFETY: as [`Self::lookup`], with no name argument.
        let code = unsafe { RHostResolver_Next(&mut self.resolver, self.answer.as_name_entry()) };
        check(code)?;
        self.answer_as_v4(port)
    }

    /// Reads `TNameRecord::iAddr` out of the answer as an IPv4 address.
    ///
    /// The stack often answers in `KAfInet6` with the address in v4-mapped form
    /// (`in_sock.h` lines 386-391), so a v4-mapped answer is converted; a genuine IPv6
    /// answer is [`InetAddr::v6_unsupported`].
    fn answer_as_v4(&mut self, port: u16) -> Result<InetAddr> {
        let addr = self.answer.addr_mut();
        // SAFETY: `addr` points at the `TSockAddr` inside the answer, at the measured
        // offset 528 of 576 bytes, 4-aligned; the resolver has just written a valid
        // address there. `Family`, `IsV4Mapped`, `ConvertToV4` and `Address` are all
        // non-leaving members taking only `this`.
        let raw = unsafe {
            match TSockAddr_Family(addr) {
                KAF_INET => TInetAddr_Address(addr),
                KAF_INET6 if TInetAddr_IsV4Mapped(addr) != 0 => {
                    TInetAddr_ConvertToV4(addr);
                    TInetAddr_Address(addr)
                }
                _ => return Err(InetAddr::v6_unsupported()),
            }
        };
        if raw == 0 {
            // A resolver that answered `KErrNone` with no address is a case nothing here
            // has produced; treat it as "not found" rather than as `0.0.0.0`.
            return Err(SymbianError::of(ErrorKind::NotFound));
        }
        Ok(InetAddr::v4(raw, port))
    }
}

impl Drop for HostResolver {
    fn drop(&mut self) {
        // SAFETY: `RHostResolver::Close` is a non-leaving member taking only `this`, and
        // it is safe on a sub-session that was never opened (both words are zero then).
        unsafe { RHostResolver_Close(&mut self.resolver) }
    }
}
