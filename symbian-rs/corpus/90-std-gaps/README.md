# 90 — closing the gaps step 77 left in `std` (2026-09-21)

Three images, all built by `symdev build` from the overlay in `symbian-rs/rust-src`:

| | bytes | source | through `symdev test --emulator` |
|---|---|---|---|
| `stdhello.exe` | 73 633 | `examples/std-hello` | `stdhello: 50 passed` |
| `stdnet.exe` | 53 359 | `examples/std-net` | `stdnet: 31 passed` |
| `spawnee.exe` | 3 208 | `examples/spawnee` | the child `std-hello` spawns |

## What became real

- **`Path` drive letters.** `sys/path/symbian/` parses `E:` as `Prefix::Disk`, so
  `is_absolute`, `parent`, `join` and `components` answer correctly about a Symbian
  path. Before this, `Path::new("C:\\x").is_absolute()` was `false` — a wrong answer
  given quietly.
- **`read_dir`** over `RFs::GetDir` and `CDir`, with the entry's attributes, size and
  name read out of the `TEntry` while the `CDir` is alive.
- **`args`** over `User::CommandLine` (not `RProcess::CommandLine`, which this SDK does
  not export), with `RProcess().FileName()` as element 0.
- **`std::net`** — TCP connect/listen/accept/read/write/shutdown, UDP bind/send/recv,
  and `RHostResolver::GetByName` behind `ToSocketAddrs`.
- **`std::process`** — `Command::spawn`, `wait`, `try_wait`, `kill`, `ExitStatus`,
  `std::process::id`; and `std::process::exit` is `User::Exit` rather than an
  undefined instruction.
- **A `CTrapCleanup` on every thread**, which nothing installed before. Without one the
  first `CleanupStack::PushL` below any framework call panics `E32USER-CBase 69` and
  kills the thread with no diagnostic at the default emulator log filter. `RFs::GetDir`
  is such a call, and that is how this was found.

## What stays `Unsupported`, each for a reason

`env` (Symbian has no environment at all — a process-local map would be a lie the first
child exposes); socket timeouts, `set_nonblocking(true)`, `peek`, `try_clone` and every
socket option; IPv6; `Stdio::piped()` and `Command::output()`, because **`RPipe` is
9.4 and is not in this SDK** — verified, not assumed: no header under
`epoc32/include` declares it; `Command::current_dir`; and `std::path::absolute`, which
needs `RFs::Parse` against the session path.

## Two emulator limitations this slice measured

- **EKA2L1 cannot spawn an image with a writable data section through the loader.**
  `RProcess::Create` succeeds, the emulator gives the child an extra `anonymous`
  0x1000-byte chunk at 0x400000, and the child dies with `KERN-EXEC 3` before `main`.
  Every `std` image tried does this; no `no_std` one does. `spawnee.exe` is `no_std`
  for exactly that reason.
- **Its `mkdir` answers `KErrAlreadyExists` where Symbian answers `KErrPathNotFound`**
  when the parent of a new directory is missing, so `create_dir_all` reads that as
  "already there" and creates nothing below the first missing level.

Every `no_std` example is unchanged to the byte: `hello-raw` 752, `hello` 3 187, `shim`
4 474, `alloc` 4 474, `files` 10 552, `atomics` 11 719, `ui` 12 715, `net` 13 379, `tls`
16 272, `time` 20 583, `async` 21 659.

What this does **not** show: a device. Every figure here is EKA2L1.

Experiment record: `docs/research/experiment-backlog.md` §90.
