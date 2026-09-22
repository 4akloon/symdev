//! `std::net` on a phone, with no `#![no_std]` and no SDK type anywhere in the file.
//!
//! `examples/net` is the same program against `symbian_std::net`, the `no_std` facade.
//! This one uses **`std::net`** — `std::net::TcpStream`, `std::io::Read`,
//! `std::net::ToSocketAddrs` — and the only Symbian thing in the project is the
//! `NetworkServices` line in `symdev.toml`.
//!
//! Nothing here is asynchronous. Every call below is `RSocket` + `TRequestStatus` +
//! `User::WaitForRequest` underneath, which is Symbian's own blocking form; no active
//! scheduler and no executor exists below this file. `std::net` refuses outright if one
//! is installed — see `sys::net::connection::symbian::request`.
//!
//! **Two peers must be running first**, both in `../net/peer/` and both described by
//! its `README.md`:
//!
//! ```sh
//! python3 ../net/peer/echoserver.py 18984 0.0.0.0 &
//! python3 ../net/peer/poker.py 18985 300 &
//! ```
//!
//! The example blocks on each in turn, so starting only one leaves it waiting and no
//! report is written — which reads as a network failure and is not one.

use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use std::net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket};

use symbian_std::test_report::{Report, detail};

mod cases;

/// The host's loopback as the emulated phone sees it — the same address, because
/// EKA2L1's socket backend is the host's own (`AF_INET` over libuv).
const HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
/// Where `../net/peer/echoserver.py` listens for this example.
const PORT: u16 = 18984;
/// Where *this* program listens; `../net/peer/poker.py` keeps trying until it succeeds.
const LISTEN_PORT: u16 = 18985;
/// What goes up the wire, with a non-ASCII byte so a truncating write would show.
const REQUEST: &[u8] = "symdev std-gaps — net through std::net\n".as_bytes();
/// What the echo server sends back for [`REQUEST`].
const EXPECTED: &[u8] = "SYMDEV STD-GAPS — NET THROUGH STD::NET-PONG\n".as_bytes();

/// One round trip: connect, write, read to the end, hand back what came.
fn round_trip(addr: impl ToSocketAddrs) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(REQUEST)?;
    let mut answer = Vec::new();
    // `read_to_end` keeps reading until the peer closes, which the server does after
    // one answer — so this also proves that end of input arrives as `Ok(0)` and not as
    // a hang. On Symbian the stack says it with `KErrEof`, not with a zero-length read.
    stream.read_to_end(&mut answer)?;
    Ok(answer)
}

/// The other direction: listen, take one connection from the host, read it and answer.
fn serve_one(listener: &TcpListener) -> std::io::Result<Vec<u8>> {
    let (mut stream, _peer) = listener.accept()?;
    let mut got = Vec::new();
    // The poker closes its write half after one line, so this ends.
    stream.read_to_end(&mut got)?;
    stream.write_all(b"served\n")?;
    Ok(got)
}

fn run(report: &mut Report) {
    let numeric = SocketAddr::V4(SocketAddrV4::new(HOST, PORT));

    cases::resolution(report, numeric, PORT);

    // The round trip, by address and by name.
    match round_trip(numeric) {
        Ok(answer) => report.check_detail(
            "round trip by address",
            answer == EXPECTED,
            detail!("{}", String::from_utf8_lossy(&answer)),
        ),
        Err(e) => report.check_detail("round trip by address", false, detail!("{e}")),
    }
    match round_trip(("localhost", PORT)) {
        Ok(answer) => report.check(
            "round trip by name, through RHostResolver",
            answer == EXPECTED,
        ),
        Err(e) => report.check_detail(
            "round trip by name, through RHostResolver",
            false,
            detail!("{e}"),
        ),
    }

    // `peer_addr`, `local_addr` and `shutdown` on a live connection.
    match TcpStream::connect(numeric) {
        Ok(stream) => {
            report.check(
                "peer_addr is where we connected",
                stream.peer_addr().ok() == Some(numeric),
            );
            report.check(
                "local_addr has a port of its own",
                stream.local_addr().map(|a| a.port() != 0).unwrap_or(false),
            );
            report.check("shutdown(Both)", stream.shutdown(Shutdown::Both).is_ok());
            cases::stream_unsupported(report, &stream);
        }
        Err(e) => report.check_detail(
            "peer_addr is where we connected",
            false,
            detail!("{e}"),
        ),
    }

    // The step-73 rule: a blocking call under an active scheduler is refused.
    cases::scheduler_refusal(report, numeric);

    // A port nothing listens on: an error, not a hang and not a success.
    report.check(
        "a closed port fails",
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(HOST, 1))).is_err(),
    );

    // This program as the server. `poker.py` is already retrying against LISTEN_PORT.
    match TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(HOST, LISTEN_PORT))) {
        Ok(listener) => {
            report.check(
                "the listener knows its port",
                listener.local_addr().map(|a| a.port()).ok() == Some(LISTEN_PORT),
            );
            match serve_one(&listener) {
                Ok(got) => report.check_detail(
                    "accept one connection and read the poke",
                    got == b"poke\n",
                    detail!("{}", String::from_utf8_lossy(&got)),
                ),
                Err(e) => report.check_detail(
                    "accept one connection and read the poke",
                    false,
                    detail!("{e}"),
                ),
            }
        }
        Err(e) => report.check_detail("bind a listener", false, detail!("{e}")),
    }

    // UDP: bind to an ephemeral port and read back which one the stack chose.
    match UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(HOST, 0))) {
        Ok(socket) => {
            report.check(
                "the stack chose a UDP port",
                socket.local_addr().map(|a| a.port() != 0).unwrap_or(false),
            );
            cases::udp_unsupported(report, &socket);
        }
        Err(e) => report.check_detail("bind a UDP socket", false, detail!("{e}")),
    }
}

#[symbian_std::main]
fn main() -> std::io::Result<()> {
    let mut report = Report::new("stdnet");
    run(&mut report);
    report.finish()?;
    Ok(())
}
