//! `std::net` for Symbian OS 9.3, over `RSocketServ`, `RSocket` and `RHostResolver`.
//!
//! This is step 74's blocking socket layer re-hosted as `std`'s own backend: the
//! behaviour was already observed and `examples/net` already proved it against a host
//! peer. What is new is that `sys::net`'s internal contract is much wider than the
//! public API — `Socket`, timeouts, `set_nonblocking`, `peek`, `try_clone`, `linger`,
//! TTL, multicast — and each of those answers `Unsupported` **individually**, with the
//! reason, rather than being approximated.
//!
//! # What is real
//!
//! | | over |
//! |---|---|
//! | `TcpStream::connect`, `read`, `write`, `shutdown`, `peer_addr`, `local_addr` | `RSocket` with `KSockStream`/`KProtocolInetTcp` |
//! | `TcpListener::bind`, `accept` | `RSocket::Bind`/`Listen`/`Accept` |
//! | `UdpSocket::bind`, `send_to`, `recv_from`, `connect`/`send`/`recv` | `RSocket` with `KSockDatagram`/`KProtocolInetUdp` |
//! | `ToSocketAddrs` for a host name | `RHostResolver::GetByName`, the **synchronous** overload |
//!
//! # The rule `std::net` inherits from step 73, and it is the important one
//!
//! Every blocking call here is `issue the request; User::WaitForRequest`. That and a
//! `CActiveScheduler` must not share a thread — they eat each other's completions —
//! so `sys::pal::symbian::request::blocking` **refuses** when a scheduler is installed rather than
//! risking a hang. An Avkon program, or anything inside `symbian_async::block_on`,
//! therefore gets an error from the first socket call, not silence. Do the socket work
//! on a `std::thread::spawn`ed thread, which has no scheduler.
//!
//! # What is `Unsupported`, and why each one is
//!
//! - **Timeouts** (`set_read_timeout`, `connect_timeout`, …) — a deadline needs an
//!   `RTimer` request outstanding *beside* the socket's, on the same thread, which is
//!   exactly what one blocking wait may not have. It is an executor's job, and the
//!   executor is `symbian_async`.
//! - **`set_nonblocking(true)`** — there is no non-blocking mode on an `RSocket`; the
//!   asynchronous form is the request/`TRequestStatus` pair, which is a different
//!   shape from a syscall that returns `EWOULDBLOCK`. `set_nonblocking(false)` is a
//!   no-op and succeeds, because blocking is what this backend already is.
//! - **`peek`** — `RSocket::Recv` takes a flags word and `es_sock.h` names
//!   `KSockReadPeek`, but nothing here has ever issued it.
//! - **`try_clone`** — `RSocket::Transfer` moves a socket to another *thread*; there is
//!   no dup of a sub-session within one.
//! - **Every socket option** — `linger`, `nodelay`, `keepalive`, TTL, broadcast,
//!   multicast. They are all `RSocket::SetOpt`/`GetOpt` with an option constant out of
//!   `in_sock.h`, and this repository has never set one and watched what changed.
//! - **IPv6** — see [`addr`].
//! - **`hostname`** — left on `sys::net::hostname`'s `unsupported` arm: there is no
//!   `gethostname` on this platform, and a phone's "name" is a Bluetooth setting, not
//!   a host name.
//!
//! # `NetworkServices`
//!
//! The socket server refuses the session with `KErrPermissionDenied` when the process
//! has no `NetworkServices` capability, before any socket exists. [`session`] turns
//! that code into a message that names the capability and the manifest key.

mod addr;
mod session;
mod socket;
mod tcp;
mod udp;

pub use tcp::{TcpListener, TcpStream};
pub use udp::UdpSocket;

use crate::io;
use crate::net::SocketAddr;
use crate::vec;
use symbian_sys::esock::{
    KAF_INET, KPROTOCOL_INET_TCP, RHostResolver, RHostResolver_Close, RHostResolver_GetByName,
    RHostResolver_Next, RHostResolver_Open, TNAME_RECORD_SIZE, TNameEntryStorage, TPckgBuf_ctor,
};

/// `KMaxHostName`: `THostName` is `TBuf<0x100>` (`es_sock.h` line 553), so a name
/// longer than 256 code units cannot be asked about and is refused before any call —
/// overflowing the descriptor would be a `USER 11` panic that no `TRAP` catches.
const MAX_HOST_NAME: usize = 0x100;

/// Everything `RHostResolver` answered for one name.
///
/// The whole list is collected before the iterator is handed out, so the resolver
/// sub-session is closed by the time the caller sees anything and `next()` cannot fail.
pub struct LookupHost(vec::IntoIter<SocketAddr>);

impl Iterator for LookupHost {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<SocketAddr> {
        self.0.next()
    }
}

/// `RHostResolver::GetByName`, then `Next` until the resolver runs out.
///
/// The **synchronous** overload — `TInt GetByName(const TDesC&, TNameEntry&)` — so a
/// name lookup needs no request status and no wait at all, and works under an active
/// scheduler where the blocking socket calls do not.
pub fn lookup_host(host: &str, port: u16) -> io::Result<LookupHost> {
    // A literal address needs no resolver, and asking one for `127.0.0.1` on a phone
    // is a round trip to whatever the stack would consult.
    if let Ok(literal) = host.parse::<crate::net::Ipv4Addr>() {
        return Ok(LookupHost(vec![SocketAddr::new(literal.into(), port)].into_iter()));
    }
    let mut resolver = Resolver::open(port)?;
    let mut found = crate::vec::Vec::new();
    let mut answer = resolver.first(host)?;
    loop {
        if let Ok(addr) = answer {
            found.push(addr);
        }
        match resolver.next() {
            Some(next) => answer = next,
            None => break,
        }
    }
    if found.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "the resolver found no IPv4 address"));
    }
    Ok(LookupHost(found.into_iter()))
}

/// An open `RHostResolver` sub-session and the packaged answer it writes into.
struct Resolver {
    resolver: RHostResolver,
    answer: TNameEntryStorage,
    port: u16,
}

impl Resolver {
    fn open(port: u16) -> io::Result<Self> {
        let mut this = Resolver {
            resolver: RHostResolver { session_handle: 0, sub_session_handle: 0 },
            answer: TNameEntryStorage::zeroed(),
            port,
        };
        // SAFETY: zeroed storage of the measured `sizeof(TNameEntry)` (576), 8-aligned
        // as `TAlignedBuf8` requires, handed to euser's own exported
        // `TBufBase8(TInt,TInt)` with the measured `sizeof(TNameRecord)` (564) for
        // both — which is what the inline `TPckgBuf<TNameRecord>()` does. The header
        // word is built by euser and never guessed here.
        unsafe {
            TPckgBuf_ctor(this.answer.as_bytes_mut(), TNAME_RECORD_SIZE, TNAME_RECORD_SIZE);
        }
        session::with_session(|s| {
            // SAFETY: `this` in argument 0 per the observed member ABI, then the
            // session (borrowed mutably, as `RSocketServ&` is) and two scalars.
            // Non-leaving; on failure the handles stay zero and `Drop` is safe.
            socket::check(unsafe {
                RHostResolver_Open(&mut this.resolver, s.as_server(), KAF_INET, KPROTOCOL_INET_TCP)
            })
        })?;
        Ok(this)
    }

    fn first(&mut self, host: &str) -> io::Result<io::Result<SocketAddr>> {
        if host.encode_utf16().count() > MAX_HOST_NAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a Symbian host name is at most KMaxHostName = 256 code units",
            ));
        }
        let name = crate::sys::pal::symbian::des::PathBuf16::new(host)?;
        // SAFETY: `this` in argument 0; the name is a live `TBuf16` borrowed and only
        // read, and `answer` is the `TPckgBuf<TNameRecord>` built above, borrowed
        // mutably. This is the synchronous overload, so it returns the `TInt` itself
        // and nothing outlives the call. Non-leaving.
        socket::check(unsafe {
            RHostResolver_GetByName(
                &mut self.resolver,
                name.as_tdesc16(),
                self.answer.as_name_entry(),
            )
        })?;
        Ok(self.answer_as_socket())
    }

    /// The next address for the same question, or `None` at the end.
    fn next(&mut self) -> Option<io::Result<SocketAddr>> {
        // SAFETY: as [`Self::first`], with no name argument.
        let code = unsafe { RHostResolver_Next(&mut self.resolver, self.answer.as_name_entry()) };
        (code == 0).then(|| self.answer_as_socket())
    }

    /// `TNameRecord::iAddr`, at the measured offset inside the answer, as a
    /// `SocketAddr`.
    fn answer_as_socket(&mut self) -> io::Result<SocketAddr> {
        let port = self.port;
        // SAFETY: `addr_mut` points at the `TSockAddr` inside the answer, at the
        // measured offset of 576 bytes, 4-aligned, and the resolver has just written a
        // valid address there. Reading it back goes through `addr::InetAddr`, which
        // only calls non-leaving `const` members on it.
        let mut addr = unsafe { addr::InetAddr::of_answer(self.answer.addr_mut()) };
        let resolved = addr.to_socket()?;
        Ok(SocketAddr::new(resolved.ip(), port))
    }
}

impl Drop for Resolver {
    fn drop(&mut self) {
        // SAFETY: `RHostResolver::Close` is a non-leaving member taking only `this`,
        // and it is safe on a sub-session that was never opened.
        unsafe { RHostResolver_Close(&mut self.resolver) };
    }
}

/// Every option this backend refuses names itself and says why, so that a program that
/// hits one reads the reason rather than a bare "unsupported".
fn no_option(name: &'static str, why: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, NoOption { name, why })
}

/// The message of [`no_option`], built without allocating a `String` for a constant.
#[derive(Debug)]
struct NoOption {
    name: &'static str,
    why: &'static str,
}

impl crate::fmt::Display for NoOption {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        write!(f, "TODO: {} (not observed) — {}", self.name, self.why)
    }
}

impl crate::error::Error for NoOption {}

/// A deadline needs a second request outstanding on the thread that is already waiting
/// for the socket's, and that is the one thing `pal::symbian::request::blocking` may not
/// have.
fn no_timeout() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "a socket timeout needs an RTimer request outstanding beside the socket's, \
         which a single User::WaitForRequest cannot have: use symbian_async, or a \
         thread of its own",
    )
}

/// There is no non-blocking mode on an `RSocket`.
fn no_nonblocking() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "an RSocket has no non-blocking mode: its asynchronous form is the \
         request/TRequestStatus pair, not a call that returns EWOULDBLOCK",
    )
}
