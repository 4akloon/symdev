# The peer `examples/net` talks to

Two short Python programs, no dependencies, nothing outside this host's loopback. Start
both before `symdev test --emulator`, because the example blocks on each in turn:

```sh
python3 peer/echoserver.py 18974 0.0.0.0 &   # the example connects here
python3 peer/poker.py 18975 300 &            # the example listens here
symdev build && symdev package && symdev test --emulator
```

`echoserver.py` answers a line with the same line uppercased and `-PONG` appended, then
closes — which is also how the example sees end of input. `poker.py` keeps retrying
against the port the example listens on until the emulated application is up, sends one
line, half-closes and prints the answer.

The emulated phone's `127.0.0.1` is **this host's** `127.0.0.1`: EKA2L1's internet
protocols hand the guest's address straight to libuv (`uv_tcp_connect`, `uv_tcp_bind`),
so there is no port mapping and no special address in either direction. That is measured,
not assumed — see [net-access-points.md](../../../docs/research/net-access-points.md).
