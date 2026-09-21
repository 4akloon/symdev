//! Networking, in `std`'s shape (design spec §6a, §11 step 74).
//!
//! The program resolves a name, opens a TCP connection, writes a line, reads the answer
//! and compares — and the only Symbian thing anywhere in it is the `NetworkServices`
//! line in `symdev.toml`. Every `use`, every type and every method below is `std`'s:
//! `net::TcpStream`, `net::ToSocketAddrs`, `io::Write::write_all`, `net::SocketAddr`.
//!
//! **Nothing here is asynchronous.** Every one of these calls is `RSocket` +
//! `TRequestStatus` + `User::WaitForRequest` underneath, which is Symbian's own blocking
//! form; no active scheduler and no executor exists below this file.
//!
//! **Two peers must be running first**, both in `peer/` beside this file and both
//! described by `peer/README.md`: `echoserver.py 18974 0.0.0.0`, which the example
//! connects to, and `poker.py 18975 300`, which connects to the listener the example
//! opens. The example blocks on each in turn, so starting only one of them leaves it
//! waiting and no report is written — which reads as a network failure and is not one.
//!
//! It reports through [`symbian_std::test_report`], which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.
#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use symbian_std::io::{self, ErrorKind, Read, Write};
use symbian_std::net::{
    Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream, ToSocketAddrs, UdpSocket,
};
use symbian_std::test_report::Report;

/// The host's loopback as the emulated phone sees it — the same address, because
/// EKA2L1's socket backend is the host's own (`AF_INET` over libuv).
const HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
/// Where `scratchpad/echoserver.py` listens.
const PORT: u16 = 18974;
/// Where *this* program listens, for the [`TcpListener`] case: `scratchpad/poker.py` on
/// the host keeps trying to connect there until it succeeds. The guest's bind is a real
/// host bind — EKA2L1 hands the address straight to `uv_tcp_bind` — so the emulated
/// application and the host agree on the port with no mapping in between.
const LISTEN_PORT: u16 = 18975;
/// What goes up the wire, with a non-ASCII byte so a truncating write would show.
const REQUEST: &[u8] = "symdev step 74 — net through symbian-std\n".as_bytes();
/// What the server sends back for [`REQUEST`].
const EXPECTED: &[u8] = "SYMDEV STEP 74 — NET THROUGH SYMBIAN-STD-PONG\n".as_bytes();

/// One round trip: connect, write, read to the end, hand back what came.
fn round_trip(addr: impl ToSocketAddrs) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(REQUEST)?;
    let mut answer = Vec::new();
    // `read_to_end` keeps reading until the peer closes, which the server does after one
    // answer — so this also proves that end of input arrives as `Ok(0)` and not as a
    // hang.
    stream.read_to_end(&mut answer)?;
    Ok(answer)
}

/// The same round trip with the peer named rather than numbered, so `RHostResolver` is
/// in the path.
fn round_trip_by_name() -> io::Result<Vec<u8>> {
    round_trip(("localhost", PORT))
}

/// The other direction: listen, take one connection from the host, read what it sent
/// and answer. `incoming()` rather than `accept()` so both are exercised.
fn serve_one(listener: &mut TcpListener) -> io::Result<Vec<u8>> {
    let mut stream = listener
        .incoming()
        .next()
        .ok_or_else(|| io::Error::from(ErrorKind::NotFound))??;
    let mut got = Vec::new();
    // The poker closes its write half after one line, so this ends.
    stream.read_to_end(&mut got)?;
    stream.write_all(b"served\n")?;
    Ok(got)
}

/// What the resolver says `localhost` is.
fn resolve(host: &str) -> io::Result<SocketAddr> {
    let mut addrs = (host, PORT).to_socket_addrs()?;
    addrs
        .next()
        .ok_or_else(|| io::Error::from(ErrorKind::NotFound))
}

fn run(report: &mut Report) {
    let numeric = SocketAddr::V4(SocketAddrV4::new(HOST, PORT));

    // 1. Resolution, three ways in: a literal through `(&str, u16)`, a name through
    //    `RHostResolver::GetByName`, and the `"host:port"` spelling.
    report.check(
        "a literal address needs no resolver",
        resolve("127.0.0.1").as_ref() == Ok(&numeric),
    );
    match report.checked("resolve a name", resolve("localhost")) {
        Some(addr) => report.check("the name is the loopback", addr == numeric),
        None => report.fail("the name is the loopback", "the lookup failed"),
    }
    match report.checked("parse \"host:port\"", "127.0.0.1:18974".to_socket_addrs()) {
        Some(mut it) => report.check("the port came from the string", it.next() == Some(numeric)),
        None => report.fail("the port came from the string", "the parse failed"),
    }
    report.check(
        "a string with no port is InvalidInput",
        "127.0.0.1".to_socket_addrs().err().map(|e| e.kind()) == Some(ErrorKind::InvalidInput),
    );

    // 2. The round trip, by address and by name.
    match report.checked("round trip by address", round_trip(numeric)) {
        Some(answer) => report.check("the bytes came back", answer == EXPECTED),
        None => report.fail("the bytes came back", "the round trip failed"),
    }
    match report.checked("round trip by name", round_trip_by_name()) {
        Some(answer) => report.check("the named round trip agrees", answer == EXPECTED),
        None => report.fail("the named round trip agrees", "the round trip failed"),
    }

    // 3. `peer_addr`, `local_addr` and `shutdown` on a live connection.
    match report.checked(
        "connect for the address checks",
        TcpStream::connect(numeric),
    ) {
        Some(mut stream) => {
            report.check(
                "peer_addr is where we connected",
                stream.peer_addr().as_ref() == Ok(&numeric),
            );
            report.check(
                "local_addr has a port of its own",
                stream.local_addr().map(|a| a.port() != 0) == Ok(true),
            );
            report.checked(
                "shutdown",
                stream.shutdown(symbian_std::net::Shutdown::Both),
            );
        }
        None => report.fail("peer_addr is where we connected", "no connection"),
    }

    // 4. A port nothing listens on: the failure has to be an error, not a hang and not
    //    a success. Which `e32err.h` code the stack picks is not asserted — that is a
    //    stack's choice and it has not been observed on a device.
    report.check(
        "a closed port fails",
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(HOST, 1))).is_err(),
    );

    // 5. IPv6 is refused rather than faked.
    report.check(
        "an IPv6 address is Unsupported",
        "[::1]:80"
            .to_socket_addrs()
            .err()
            .map(|e| e.kind())
            .is_some(),
    );

    // 6. The other direction: this program as the server. The host's `poker.py` is
    //    already retrying against `LISTEN_PORT` when the emulator starts.
    match report.checked(
        "bind a listener",
        TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(HOST, LISTEN_PORT))),
    ) {
        Some(mut listener) => {
            report.check(
                "the listener knows its port",
                listener.local_addr().map(|a| a.port()) == Ok(LISTEN_PORT),
            );
            match report.checked("accept one connection", serve_one(&mut listener)) {
                Some(got) => report.check("the poke arrived", got == b"poke\n"),
                None => report.fail("the poke arrived", "no connection was served"),
            }
        }
        None => report.fail("accept one connection", "the bind failed"),
    }

    // 7. UDP, which came free with the same `RSocket`: bind to an ephemeral port and
    //    read back which one the stack chose.
    match report.checked(
        "bind a UDP socket",
        UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(HOST, 0))),
    ) {
        Some(mut socket) => report.check(
            "the stack chose a UDP port",
            socket.local_addr().map(|a| a.port() != 0) == Ok(true),
        ),
        None => report.fail("the stack chose a UDP port", "the bind failed"),
    }
}

#[symbian_std::main]
fn main() -> io::Result<()> {
    let mut report = Report::new("net");
    run(&mut report);
    report.finish()?;
    Ok(())
}
