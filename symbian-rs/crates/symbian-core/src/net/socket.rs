//! `Socket`: one endpoint (`RSocket`).
//!
//! Nothing here leaves (`es_sock.h` declares no leaving member on `RSocket`), so there is
//! no C++ in the path. The handle is closed when the value is dropped.
//!
//! The operations that take a `TRequestStatus&` go through [`super::request::blocking`],
//! which issues the request and waits for it on the calling thread. That is the blocking
//! form Symbian itself documents, and it is what `std::net` means; no active scheduler
//! exists anywhere below this type.
use symbian_sys::esock::{
    ESHUTDOWN_IMMEDIATE, ESHUTDOWN_NORMAL, ESHUTDOWN_STOP_INPUT, ESHUTDOWN_STOP_OUTPUT, KAF_INET,
    KPROTOCOL_INET_TCP, KPROTOCOL_INET_UDP, KSOCK_DATAGRAM, KSOCK_STREAM, RSocket, RSocket_Accept,
    RSocket_Bind, RSocket_Close, RSocket_Connect, RSocket_Listen, RSocket_LocalName, RSocket_Open,
    RSocket_OpenBlank, RSocket_RecvFrom, RSocket_RecvOneOrMore, RSocket_RemoteName, RSocket_Send,
    RSocket_SendTo, RSocket_Shutdown, TPckgBuf_ctor, TSockXfrLengthStorage,
};

use super::addr::InetAddr;
use super::request::blocking;
use super::server::SocketServer;
use crate::ErrorKind;
use crate::des8::{DesC8, Ptr8, PtrC8};
use crate::error::{Result, check};

/// Which half of a connection to stop (`RSocket::TShutdown`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// `EStopInput`: stop reading.
    Read,
    /// `EStopOutput`: stop writing, which is what sends the FIN.
    Write,
    /// `ENormal`: stop both and complete when the connection is closed gracefully.
    Both,
    /// `EImmediate`: abortive close, with no graceful exchange.
    Immediate,
}

impl Shutdown {
    const fn how(self) -> i32 {
        match self {
            Self::Read => ESHUTDOWN_STOP_INPUT,
            Self::Write => ESHUTDOWN_STOP_OUTPUT,
            Self::Both => ESHUTDOWN_NORMAL,
            Self::Immediate => ESHUTDOWN_IMMEDIATE,
        }
    }
}

/// An open socket.
///
/// Not `Send` or `Sync`: a sub-session handle belongs to the thread that opened it.
pub struct Socket {
    socket: RSocket,
}

impl Socket {
    const fn closed() -> Self {
        Self {
            socket: RSocket {
                session_handle: 0,
                sub_session_handle: 0,
            },
        }
    }

    /// A TCP socket over IPv4 (`RSocket::Open(server, KAfInet, KSockStream,
    /// KProtocolInetTcp)`).
    pub fn tcp(server: &mut SocketServer) -> Result<Self> {
        Self::opened(server, KSOCK_STREAM, KPROTOCOL_INET_TCP)
    }

    /// A UDP socket over IPv4 (`RSocket::Open(server, KAfInet, KSockDatagram,
    /// KProtocolInetUdp)`).
    pub fn udp(server: &mut SocketServer) -> Result<Self> {
        Self::opened(server, KSOCK_DATAGRAM, KPROTOCOL_INET_UDP)
    }

    fn opened(server: &mut SocketServer, sock_type: u32, protocol: u32) -> Result<Self> {
        let mut socket = Self::closed();
        // SAFETY: `this` in argument 0 per the observed member ABI, then the session
        // (borrowed mutably, as `RSocketServ&` is) and three scalars. Non-leaving, and
        // every failure is the returned `TInt`; on failure the handles stay zero and
        // `Drop` is safe on those.
        let code = unsafe {
            RSocket_Open(
                &mut socket.socket,
                server.as_server(),
                KAF_INET,
                sock_type,
                protocol,
            )
        };
        check(code)?;
        Ok(socket)
    }

    /// A socket with no protocol (`RSocket::Open(RSocketServ&)`), which is the only
    /// thing [`Socket::accept`] may be handed as its destination.
    pub fn blank(server: &mut SocketServer) -> Result<Self> {
        let mut socket = Self::closed();
        // SAFETY: as [`Self::opened`], with one fewer argument.
        let code = unsafe { RSocket_OpenBlank(&mut socket.socket, server.as_server()) };
        check(code)?;
        Ok(socket)
    }

    /// Connects to `addr` and blocks until the stack answers (`RSocket::Connect`).
    pub fn connect(&mut self, addr: &mut InetAddr) -> Result<()> {
        let socket = &raw mut self.socket;
        let addr = addr.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: `this` in argument 0; the address is a live `TInetAddr` borrowed
            // mutably for the call (Symbian takes it non-`const` although it only reads
            // it), and the status is the live one `blocking` waits for immediately
            // afterwards. Non-leaving.
            unsafe { RSocket_Connect(socket, addr, status) };
        })
        .map(|_| ())
    }

    /// Binds to a local address (`RSocket::Bind`).
    pub fn bind(&mut self, addr: &mut InetAddr) -> Result<()> {
        // SAFETY: `this` in argument 0 and a live `TInetAddr`. Synchronous and
        // non-leaving; the failure is the returned `TInt`.
        let code = unsafe { RSocket_Bind(&mut self.socket, addr.as_sockaddr_mut()) };
        check(code).map(|_| ())
    }

    /// Starts listening with room for `backlog` pending connections
    /// (`RSocket::Listen`).
    pub fn listen(&mut self, backlog: u32) -> Result<()> {
        // SAFETY: `this` in argument 0 and a scalar. Non-leaving.
        let code = unsafe { RSocket_Listen(&mut self.socket, backlog) };
        check(code).map(|_| ())
    }

    /// Waits for a connection and moves it into `blank` (`RSocket::Accept`).
    ///
    /// `blank` must have come from [`Socket::blank`] on the same session; Symbian
    /// panics a client that hands it anything else.
    pub fn accept(&mut self, blank: &mut Self) -> Result<()> {
        let socket = &raw mut self.socket;
        let blank = &raw mut blank.socket;
        blocking(|status| {
            // SAFETY: `this` in argument 0, then the blank socket (a live, open
            // `RSocket` borrowed mutably for the call) and the status `blocking` waits
            // for. Non-leaving.
            unsafe { RSocket_Accept(socket, blank, status) };
        })
        .map(|_| ())
    }

    /// Sends all of `buf` and blocks until the stack has taken it (`RSocket::Send`).
    ///
    /// Symbian sends the whole descriptor or fails, so there is no short write here and
    /// nothing for a caller to loop over — the same shape `RFile::Write` has.
    pub fn send(&mut self, buf: &[u8]) -> Result<()> {
        let des = PtrC8::new(buf)?;
        let socket = &raw mut self.socket;
        let des = des.as_tdesc8();
        blocking(|status| {
            // SAFETY: `this` in argument 0; the descriptor is a real `TPtrC8` built by
            // euser over `buf`, which stays borrowed until this function returns — and
            // `blocking` does not return until the send has completed, so the stack is
            // never left holding a pointer into freed memory.
            unsafe { RSocket_Send(socket, des, 0, status) };
        })
        .map(|_| ())
    }

    /// Reads whatever has arrived, up to `buf.len()`, and returns how many bytes that
    /// was (`RSocket::RecvOneOrMore`).
    ///
    /// `RecvOneOrMore` and not `Recv`: `Recv` does not complete until the descriptor is
    /// full, which is not what `std::io::Read` promises.
    ///
    /// **End of input is `KErrEof` (-25), and that is observed**: reading from a peer
    /// that had closed its end completed the request with `KErrEof` and not with zero
    /// bytes (`examples/net`, 2026-09-20, where it first arrived as a failed
    /// `read_to_end`). Since `KErrEof` says only "no more will come", it is reported
    /// here as the bytes that did arrive — which is `0` when none did. That is the same
    /// statement `RFile::Read` makes for end of file, so the two ends of this crate say
    /// end of input the same way, and no byte the stack delivered alongside it is lost.
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut des = Ptr8::new(buf)?;
        let mut xfr = TSockXfrLengthStorage::zeroed();
        // SAFETY: zeroed storage of the measured `sizeof(TSockXfrLength)` (16),
        // 8-aligned as `TAlignedBuf8` requires, handed to euser's own exported
        // `TBufBase8(TInt,TInt)` with `sizeof(TInt)` for both — which is exactly what
        // the inline `TPckgBuf<TInt>()` does (`e32cmn.inl` lines 2659 and 1179). So the
        // descriptor header is built by euser and never guessed here.
        unsafe {
            TPckgBuf_ctor(
                (&raw mut xfr).cast(),
                TSockXfrLengthStorage::payload_len(),
                TSockXfrLengthStorage::payload_len(),
            );
        }
        let socket = &raw mut self.socket;
        let buf_des = des.as_tdes8();
        let xfr_des = xfr.as_tdes8();
        let outcome = blocking(|status| {
            // SAFETY: `this` in argument 0; `buf_des` is a real `TPtr8` over the
            // caller's slice with `iMaxLength` equal to its length, so every byte the
            // stack writes is inside it; `xfr_des` is the packaged `TInt` the client
            // stub carries the flags in and writes the count back to. Both stay alive
            // until `blocking` returns, which is after the request completed.
            unsafe { RSocket_RecvOneOrMore(socket, buf_des, 0, status, xfr_des) };
        });
        match outcome {
            Ok(_) => Ok(des.len()),
            Err(e) if e.kind() == ErrorKind::Eof => Ok(des.len()),
            Err(e) => Err(e),
        }
    }

    /// Sends one datagram to `addr` (`RSocket::SendTo`).
    pub fn send_to(&mut self, buf: &[u8], addr: &mut InetAddr) -> Result<()> {
        let des = PtrC8::new(buf)?;
        let socket = &raw mut self.socket;
        let des = des.as_tdesc8();
        let addr = addr.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: as [`Self::send`], with a live `TInetAddr` in argument 2.
            unsafe { RSocket_SendTo(socket, des, addr, 0, status) };
        })
        .map(|_| ())
    }

    /// Receives one datagram and the address it came from (`RSocket::RecvFrom`).
    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, InetAddr)> {
        let mut des = Ptr8::new(buf)?;
        let mut from = InetAddr::blank();
        let socket = &raw mut self.socket;
        let buf_des = des.as_tdes8();
        let from_addr = from.as_sockaddr_mut();
        blocking(|status| {
            // SAFETY: as [`Self::recv`]; `from_addr` is zeroed storage of the measured
            // `sizeof(TSockAddr)` that the stack fills in, and it outlives the wait.
            unsafe { RSocket_RecvFrom(socket, buf_des, from_addr, 0, status) };
        })?;
        Ok((des.len(), from))
    }

    /// Stops one or both halves of the connection and blocks until it is done
    /// (`RSocket::Shutdown`).
    pub fn shutdown(&mut self, how: Shutdown) -> Result<()> {
        let socket = &raw mut self.socket;
        let how = how.how();
        blocking(|status| {
            // SAFETY: `this` in argument 0, then the enum as a scalar `int` and the
            // status. Non-leaving.
            unsafe { RSocket_Shutdown(socket, how, status) };
        })
        .map(|_| ())
    }

    /// This socket's own address (`RSocket::LocalName`), which is how a listener bound
    /// to port 0 learns which port it got.
    pub fn local_addr(&mut self) -> InetAddr {
        let mut addr = InetAddr::blank();
        // SAFETY: `this` in argument 0 and zeroed storage of the measured
        // `sizeof(TSockAddr)` that the call fills in. It returns `void`: there is no
        // error path, and an unbound socket simply reads back as `KAFUnspec`.
        unsafe { RSocket_LocalName(&mut self.socket, addr.as_sockaddr_mut()) };
        addr
    }

    /// The peer's address (`RSocket::RemoteName`).
    pub fn peer_addr(&mut self) -> InetAddr {
        let mut addr = InetAddr::blank();
        // SAFETY: as [`Self::local_addr`].
        unsafe { RSocket_RemoteName(&mut self.socket, addr.as_sockaddr_mut()) };
        addr
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        // SAFETY: `RSocket::Close` is a non-leaving member taking only `this`, and it is
        // safe on a socket that was never opened (both words are zero then).
        unsafe { RSocket_Close(&mut self.socket) }
    }
}
