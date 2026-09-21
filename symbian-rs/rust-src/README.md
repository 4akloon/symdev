# A patched `std` for `arm-symbian-e32`

This directory is the whole difference between the standard library rustup installs and
the one a `language = "rust-std"` project is built against (design spec §11 step 77,
experiments 89 and 90). It is **not** a fork of `rust-src`: it is an overlay of some
forty files, and the other 82 MB come from the toolchain at build time.

## How a build uses it

`symdev build` does this itself, in `StdSrc::materialise`
(`crates/symdev-build/src/std_src.rs`):

1. `rustc --print sysroot` names the pinned nightly; its
   `lib/rustlib/src/rust/library` is copied to `<project>/build/rust-src/library`.
   (`SYMDEV_RUST_STD_SRC` moves that copy somewhere shared.)
2. `symbian-rs/crates/symbian-sys` is copied in as `library/symbian-sys`.
3. `overlay/library/` is copied over the top.
4. cargo is run with `__CARGO_TESTS_ONLY_SRC_ROOT=<copy>/library` and
   `-Zbuild-std=std,panic_abort`.

Nothing in the rustup component is touched; it is shared between every project on the
host and rustup overwrites it on the next update.

By hand, the same thing is four commands:

```sh
SRC=$(rustc --print sysroot)/lib/rustlib/src/rust/library
cp -a "$SRC" /tmp/symbian-std/library
cp -a symbian-rs/crates/symbian-sys /tmp/symbian-std/library/symbian-sys
cp -R symbian-rs/rust-src/overlay/library/. /tmp/symbian-std/library/
__CARGO_TESTS_ONLY_SRC_ROOT=/tmp/symbian-std/library \
  cargo build --release --target symbian-rs/targets/arm-symbian-e32.json \
  -Zbuild-std=std,panic_abort -Zjson-target-spec
```

Two things about doing it by hand. `cp -a` keeps the source's modification times, and
cargo decides what to rebuild from those, so a re-materialised tree can be called
unchanged — `touch` the copied `library/std` and `library/symbian-sys`, or delete the
`build/<hash>` directories for them. `StdSrc` writes every file afresh and does not
have the problem.

## Why `__CARGO_TESTS_ONLY_SRC_ROOT`

It is cargo's own test hook, and it is the only thing that works. The obvious
alternative, `rustup toolchain link` to a directory whose `lib/rustlib/src` is the
patched copy, does **not**: a symlinked `bin/rustc` resolves `/proc/self/exe` back to
the real toolchain, so `rustc --print sysroot` answers with the original and the patched
source is never read. Making it work would need `bin` hard-linked or copied onto the
same filesystem as `~/.rustup` — more machinery for the same result. The variable is
"tests only", but `rust-toolchain.toml` pins the nightly, so the cargo that reads it is
pinned too.

## What is in the overlay

Two kinds of file, and `overlay.toml` is what tells them apart.

**Files the overlay adds.** The platform layer itself, which no toolchain will ever
conflict with:

| | |
|---|---|
| `std/src/sys/pal/symbian/` | process start and end, the descriptor helpers (`des.rs`) the other backends share, the per-thread `CTrapCleanup` (`cleanup.rs`) and the one blocking request/wait (`request.rs`) |
| `std/src/sys/alloc/symbian/` | `User::Alloc`/`Free`/`ReAlloc`, and the heap lock that goes on when the first thread is created |
| `std/src/sys/args/symbian.rs` | `User::CommandLine`, split on whitespace, with the image's own path first |
| `std/src/sys/env/symbian.rs` | there is no environment: an empty iterator, and the reason |
| `std/src/sys/fs/symbian/` | `RFs`/`RFile`, one session per thread, and `read_dir` over `RFs::GetDir` |
| `std/src/sys/net/connection/symbian/` | `RSocketServ`/`RSocket`/`RHostResolver`, blocking, one session per thread |
| `std/src/sys/path/symbian/` | backslash separators and a drive letter as `Prefix::Disk` |
| `std/src/sys/process/symbian/` | `RProcess::Create`/`Resume`/`Logon`; no stdio, because there is no `RPipe` |
| `std/src/sys/io/error/symbian.rs` | `e32err.h` codes as `io::ErrorKind`, with the `KErr*` names |
| `std/src/sys/stdio/symbian.rs` | where `println!` goes: `E:\symdev\stdout.txt` |
| `std/src/sys/sync/{mutex,condvar,thread_parking}/symbian.rs`, `sys/sync/lazy_handle.rs` | `RSemaphore`, `RCondVar` and an `RMutex` to pair with it |
| `std/src/sys/thread/symbian.rs` | `RThread`, and the thread-local sweep nothing in the kernel does |
| `std/src/sys/thread_local/key/symbian.rs` | `UserSvr::DllTls`, one slot per key |
| `std/src/sys/time/symbian.rs` | `User::TickCount` and `TTime::UniversalTime` |
| `std/src/os/symbian/mod.rs` | `std::os::symbian::start`, the entry point `#[symbian_std::main]` calls |
| `symbian-sys/Cargo.toml` | the manifest the SDK crate gets inside `library/` |

**Files the overlay replaces**, each a copy of one of `std`'s own with one `cfg_select!`
arm added — or, for `std/build.rs`, `target_os == "symbian"` added to the list of
platforms that are not `restricted_std`, for `std/Cargo.toml`, the `symbian-sys`
dependency, for `std/src/rt.rs`, the `symbian_start` entry point, and for
`std/src/sys/exit.rs`, `User::Exit` instead of the fallback arm's undefined
instruction. Those are the ones `overlay.toml` records a SHA-1 for.

## Bumping the nightly

The recorded hashes are what make that safe. Materialising checks every one, and a
changed original stops the build and names the file. Then, for each:

```sh
diff -u $(rustc --print sysroot)/lib/rustlib/src/rust/library/<path> \
        symbian-rs/rust-src/overlay/library/<path>
```

carry the upstream change into the overlay copy, and record the new hash. The added
files need no attention unless the facility's contract itself changed, which shows up
as a compile error rather than silently.
