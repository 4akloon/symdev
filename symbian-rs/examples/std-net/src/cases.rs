//! The cases that do not need the peers: name resolution, and every part of
//! `std::net`'s surface that this platform answers `Unsupported` to.
//!
//! The second half matters as much as the first. A backend that quietly *succeeded* at
//! `set_read_timeout` and then blocked forever would be worse than one that refuses,
//! so each refusal is a case here and a regression would show up as a failure rather
//! than as a hang on a device.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use symbian_std::test_report::{Report, detail};

/// `ToSocketAddrs`, three ways in: a literal, a name through `RHostResolver::GetByName`
/// and the `"host:port"` spelling.
pub fn resolution(report: &mut Report, numeric: SocketAddr, port: u16) {
    report.check_detail(
        "a literal address needs no resolver",
        ("127.0.0.1", port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut a| a.next())
            == Some(numeric),
        detail!("{numeric}"),
    );
    let named = ("localhost", port).to_socket_addrs().map(|mut a| a.next());
    report.check_detail(
        "RHostResolver::GetByName resolves a name",
        named.as_ref().ok().and_then(|a| *a) == Some(numeric),
        detail!("{named:?}"),
    );
    report.check(
        "\"host:port\" parses and keeps the port",
        "127.0.0.1:18984"
            .to_socket_addrs()
            .ok()
            .and_then(|mut a| a.next())
            == Some(numeric),
    );
    report.check(
        "a string with no port is InvalidInput",
        "127.0.0.1".to_socket_addrs().err().map(|e| e.kind()) == Some(ErrorKind::InvalidInput),
    );
    report.check_detail(
        "a name that resolves to nothing is an error",
        ("no-such-host.invalid", port).to_socket_addrs().is_err(),
        detail!(
            "{:?}",
            ("no-such-host.invalid", port).to_socket_addrs().err()
        ),
    );
    // IPv6 is refused rather than faked: there is no `TInetAddr::SetAddress(TIp6Addr)`
    // in this SDK because nothing has ever watched one work.
    report.check_detail(
        "an IPv6 peer is Unsupported, not a wrong answer",
        TcpStream::connect("[::1]:80").is_err(),
        detail!(
            "{:?}",
            TcpStream::connect("[::1]:80").err().map(|e| e.kind())
        ),
    );
}

/// Everything `sys::net`'s contract asks a `TcpStream` for that this platform will not
/// pretend to do.
pub fn stream_unsupported(report: &mut Report, stream: &TcpStream) {
    let unsupported =
        |r: std::io::Result<()>| r.err().map(|e| e.kind()) == Some(ErrorKind::Unsupported);
    report.check(
        "set_read_timeout is Unsupported: a deadline needs a second outstanding request",
        unsupported(stream.set_read_timeout(Some(Duration::from_secs(1)))),
    );
    report.check(
        "set_write_timeout is Unsupported for the same reason",
        unsupported(stream.set_write_timeout(Some(Duration::from_secs(1)))),
    );
    report.check(
        "read_timeout with none set is None",
        stream.read_timeout().ok() == Some(None),
    );
    report.check(
        "set_nonblocking(true) is Unsupported: an RSocket has no such mode",
        unsupported(stream.set_nonblocking(true)),
    );
    report.check(
        "set_nonblocking(false) succeeds, because blocking is what this is",
        stream.set_nonblocking(false).is_ok(),
    );
    report.check_detail(
        "peek is Unsupported: KSockReadPeek has not been observed",
        stream.peek(&mut [0u8; 4]).is_err(),
        detail!("{:?}", stream.peek(&mut [0u8; 4]).err()),
    );
    report.check_detail(
        "try_clone is Unsupported: a sub-session cannot be duplicated in a thread",
        stream.try_clone().is_err(),
        detail!("{:?}", stream.try_clone().err()),
    );
    report.check(
        "set_nodelay is Unsupported",
        unsupported(stream.set_nodelay(true)),
    );
    // `set_linger` is still unstable (`tcp_linger`, rust#88494), so it is not called
    // here; the backend refuses it, like every other socket option.
    report.check("set_ttl is Unsupported", unsupported(stream.set_ttl(4)));
    report.check(
        "take_error is None: every Symbian call reports at the call",
        matches!(stream.take_error(), Ok(None)),
    );
}

/// The step-73 rule `std::net` inherits: a blocking socket call and a
/// `CActiveScheduler` must not share a thread, because they eat each other's
/// completions. The backend checks `CActiveScheduler::Current()` and **refuses**, so
/// an Avkon application or anything inside `symbian_async::block_on` gets an error
/// where it would otherwise get a hang with no diagnostic.
///
/// This is the one case in the example that reaches for a Symbian type, and it does so
/// only to install the thing that must be refused.
pub fn scheduler_refusal(report: &mut Report, addr: SocketAddr) {
    let mut scheduler = core::ptr::null_mut();
    // SAFETY: the shim installs one `CActiveScheduler` on this thread and hands back
    // the pointer to uninstall it with; both are null-safe and neither leaves (the
    // TRAP is inside the shim). Nothing between the two calls starts the scheduler or
    // creates an active object.
    let installed = unsafe { symbian_sys::active::symrs_scheduler_install(&mut scheduler) };
    if installed != 0 {
        report.check_detail(
            "install a CActiveScheduler for the refusal case",
            false,
            detail!("KErr {installed}"),
        );
        return;
    }
    let refused = TcpStream::connect(addr);
    let kind = refused.as_ref().err().map(|e| e.kind());
    // SAFETY: the pointer the install just handed back, uninstalled once.
    unsafe { symbian_sys::active::symrs_scheduler_uninstall(scheduler) };
    report.check_detail(
        "a blocking connect under a CActiveScheduler is refused, not hung",
        kind == Some(ErrorKind::ResourceBusy),
        detail!("{kind:?}"),
    );
    report.check("and the socket is not returned", refused.is_err());
}

/// The same for a `UdpSocket`, where the whole option surface is `RSocket::SetOpt`.
pub fn udp_unsupported(report: &mut Report, socket: &UdpSocket) {
    let unsupported =
        |r: std::io::Result<()>| r.err().map(|e| e.kind()) == Some(ErrorKind::Unsupported);
    report.check(
        "set_broadcast is Unsupported",
        unsupported(socket.set_broadcast(true)),
    );
    report.check("set_ttl is Unsupported", unsupported(socket.set_ttl(4)));
    report.check(
        "join_multicast_v4 is Unsupported",
        unsupported(socket.join_multicast_v4(&Ipv4Addr::new(239, 0, 0, 1), &Ipv4Addr::UNSPECIFIED)),
    );
    report.check(
        "peer_addr with no connect is NotConnected",
        socket.peer_addr().err().map(|e| e.kind()) == Some(ErrorKind::NotConnected),
    );
}
