# What `std::net` cannot say on a phone: access points, capabilities and the emulator

Step 74 gives symdev a `symbian_std::net` that looks exactly like `std::net` — `TcpStream`,
`TcpListener`, `UdpSocket`, `ToSocketAddrs`, `core::net::SocketAddr`. Most of Symbian's
socket API maps onto that shape without argument. Three things do not, and §6a of the
[design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) says to name them rather
than hide them behind an abstraction the first failure would expose.

## 1. Choosing an access point has no `std` equivalent

`RSocket::Open` and `RHostResolver::Open` each have two overloads that matter here:

```cpp
IMPORT_C TInt Open(RSocketServ& aServer, TUint addrFamily, TUint sockType, TUint protocol);
IMPORT_C TInt Open(RSocketServ& aServer, TUint addrFamily, TUint sockType, TUint protocol,
                   RConnection& aConnection);          // es_sock.h lines 762-763
```

The second names an **`RConnection`**, which is Symbian's handle on a started
connection — in practice an IAP (Internet Access Point) out of CommDb: a GPRS or 3G
bearer with an APN, user name and password, or a configured WLAN. A phone has several
and no notion of "the" network: which one a socket uses is a decision, and on S60 3rd
edition it is usually made by `RConnection::Start` with a `TCommDbConnPref`, or left to
the socket server, which then either uses the device's default connection or raises the
"Select access point" dialog the user knows.

`symbian_std::net` uses the **first** overload. So:

- On a device with a default connection configured, it works and the user may never see
  a dialog.
- On a device without one, the socket server's behaviour is **untested here** — it may
  prompt, it may fail, and which it does depends on the phone's configuration. Nothing in
  this repository has observed it, so nothing claims it.
- The choice itself is unreachable from the `std`-shaped API, deliberately. There is no
  `std` word for it, and inventing `TcpStream::connect_via(ap)` would be a Symbian
  concept wearing a `std` name.

When an application genuinely needs to pick, it will get a Symbian-named type in a
Symbian-named module (`symbian_core::net::Connection` over `RConnection`), the way the
descriptor types stayed reachable under `symbian_core::des`. That is the escape hatch,
not the road.

## 2. `NetworkServices` is a build-time fact, and the emulator will not remind you

The SDK header states the requirement precisely, and on `RSocket::Open` rather than on
the session:

> `in_sock.h` lines 66 and 74 — `@capability NetworkServices Required for opening 'tcp'
> sockets. @ref RSocket::Open()` (and the same for `'udp'`).

**EKA2L1 does not enforce it.** Observed on 2026-09-20: `examples/net`, rebuilt with
`capabilities = []` in `symdev.toml`, still resolved a name and completed a TCP round
trip in the emulator. So a developer who forgets the manifest line gets a green emulator
run and a bare `-46` on a phone with no console.

That is why the check moved to build time. `symdev build` reads the linked ELF's
`DT_NEEDED` list — which only names `esock`/`insock` when the program really reached for a
socket, because the SDK puts them on every Rust link line under `--as-needed` (experiment
78) — and refuses to write the E32 when `[symbian] capabilities` does not grant what the
imports need:

```
error: netdemo.exe imports esock{000a0000}[10003d3f].dll, which needs the
`NetworkServices` capability, but `[symbian] capabilities` in symdev.toml does not grant
it. Add it:

    [symbian]
    capabilities = ["NetworkServices"]

Why: in_sock.h lines 66 and 74: `@capability NetworkServices Required for opening 'tcp'
sockets. @ref RSocket::Open()`, and the same for 'udp'. It is a user-grantable
capability, so a self-signed SIS may carry it and the phone asks the user at install
time.
```

The rule lives in `crates/symdev-build/src/required_capability.rs` and runs for Rust
builds only: a `.mmp` project already declares its capabilities in the `.mmp` and
`MmpCapabilities::check` cross-checks those against the manifest, while a Rust project has
no `.mmp` and the SDK rather than the project decides what is linked.

## 3. What a green emulator run does and does not prove

EKA2L1's socket server is real and its internet protocols are a bridge to the host's own
networking: `src/emu/services/src/internet/protocols/` opens `AF_INET`/`AF_INET6` host
sockets over libuv and resolves through `getaddrinfo`, and both TCP and UDP are registered
unconditionally at start-up. Observed here: the emulated app connected to
`127.0.0.1:18974` and the Python server on **this host's** loopback logged the connection.
No port mapping, no special address — the guest's `127.0.0.1` is the host's.

So a passing `examples/net` says: the mangled names are right, the measured layouts are
right, `TRequestStatus` + `User::WaitForRequest` is enough, the descriptors are built
correctly, and the `std`-shaped API does what it says.

It says **nothing** about:

- whether a device connects at all, which is the access-point question above;
- the capability, which the emulator ignores;
- how long anything takes, or what happens when a bearer drops mid-transfer — there is no
  GPRS, no roaming and no signal in the emulator;
- which `e32err.h` code a real stack returns for a refused connection or a dead peer.
  `examples/net` therefore asserts that connecting to a closed port *fails*, and does not
  assert *which* code comes back.

The same caveat experiment 79 recorded for the file server applies here in a stronger
form: the emulator is a faithful enough implementation of the **API** to catch a wrong
symbol or a wrong offset, and it is not a phone.
