# 69 — hello with no `unsafe` (2026-09-20)

`hello.exe`, 10 375 bytes, is `symbian-rs/examples/hello` after the rewrite onto
`symbian-core`: the note is built with `write!` into a `Buf16<64>` and shown through
`symbian_core::user::info_print`. The source contains no `unsafe` block and names no C
function. In EKA2L1:

```
[Service.Notifier]: Trying to display: Hello from Rust SDK (19 chars)
```

`helloraw.exe`, 752 bytes, is `symbian-rs/examples/hello-raw` — the experiment-65 raw
version kept as the regression. It is byte-for-byte the same size as the E32 recorded in
experiment 65, which is what shows that `--gc-sections` costs a minimal program nothing:

```
[Service.Notifier]: Trying to display: Hello from Rust SDK
```

Experiment record: `docs/research/experiment-backlog.md` §69.
