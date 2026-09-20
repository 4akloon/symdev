# 78 — the C++ shim: a leave that comes back as a value (2026-09-20)

`shimdemo.exe`, 4 475 bytes, is `symbian-rs/examples/shim`. It calls a Symbian API that
leaves, through the SDK's C++ shim, and lives to report the code; it calls a non-leaving
one directly, with no C++ anywhere in the path. In EKA2L1:

```
[Service.Notifier]: Trying to display: shim70 mkdirall=0 trapped=-12 bad=0 ensured=0 sign=-42 alive
```

- `mkdirall=0` — `RFs::MkDirAll`, a **non-static member function that cannot leave**,
  called straight from Rust with `this` as argument 0.
- `trapped=-12` — `User::LeaveIfError(-12)` really left, inside the shim's `TRAP`, and
  `KErrNotFound` arrived in Rust as an `Err`. Everything printed after it is the proof
  that the process survived.
- `bad=0 ensured=0` — `BaflUtils::EnsurePathExistsL` through the same shim. EKA2L1's
  file server accepts paths a phone would refuse (`Z:`, `Y:`, `Q:`, a `*` in a
  component all returned `KErrNone`), so this call does not leave here.
- `sign=-42` — euser's `TDes16::AppendNum(TInt64)` renders the minus sign, which is what
  the wrapper's capacity check assumes.

The counter-proof is not in the corpus, by design: the same image with the `TRAP` removed
stops at the note before the leave and the process disappears with nothing after it in
the log — no panic, no `KERN-EXEC`, no line.

Sizes unchanged by this step: `examples/hello` 3 187 bytes, `examples/hello-raw` 752.

Experiment record: `docs/research/experiment-backlog.md` §78.
