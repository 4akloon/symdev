# 84 — networking in `std`'s shape, with no executor (2026-09-21)

`netdemo.exe`, 13 183 bytes, is `symbian-rs/examples/net`. It resolves a name, opens TCP
connections in both directions and binds a UDP socket, using only `std`'s names —
`net::TcpStream`, `net::TcpListener`, `net::UdpSocket`, `net::ToSocketAddrs`,
`core::net::SocketAddr`, `io::Read`, `io::Write`. Through `symdev test --emulator`:

```
netdemo: 22 passed
```

What that covers, in the order the example runs it:

- **Resolution** — a literal through `(&str, u16)`, `localhost` through
  `RHostResolver::GetByName`'s *synchronous* overload (`2130706433` = `127.0.0.1`), the
  `"host:port"` spelling, and a string with no port as `InvalidInput`.
- **A TCP round trip, by address and by name**, against `peer/echoserver.py` on the build
  host's loopback: 40 bytes up, 45 back, em-dash included, compared byte for byte.
- **`peer_addr`, `local_addr`, `shutdown`** on a live connection, and a connection to a
  port nothing listens on failing rather than hanging.
- **IPv6 refused** rather than faked.
- **The other direction**: the example binds a `TcpListener` on `127.0.0.1:18975`, takes a
  connection from `peer/poker.py` on the host through `incoming()`, reads the line and
  answers — the host logged `answer b'served\n'`.
- **UDP** bound to an ephemeral port, with the stack's choice read back.

**No active scheduler and no executor is anywhere in this image.** Every `RSocket`
operation here is the Symbian asynchronous pair — the request, then
`User::WaitForRequest` — which is the blocking form `std::net` means. Step 73's async
layer will sit beside these same `symbian-sys` declarations rather than under them.

**No C++ shim either.** `es_sock.h` declares no leaving member on `RSocketServ`,
`RSocket` or `RHostResolver`, and `in_sock.h` declares none at all, so every call is made
directly with `this` as argument 0 (the member ABI of experiment 78).

Sizes unchanged by this step: `hello` 3 187, `hello-raw` 752, `alloc` 4 320, `shim`
4 474, `files` 10 423 — adding `esock.dso` and `insock.dso` to every Rust link line costs
nothing, because `--as-needed` drops a DSO no symbol references.

What this does **not** show: a device. EKA2L1 does not enforce `NetworkServices` (the
same example passes there with an empty capability list) and has no access point, no
bearer and no signal. See [net-access-points.md](../../docs/research/net-access-points.md).

Experiment record: `docs/research/experiment-backlog.md` §84.
