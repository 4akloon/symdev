# A patched `std` for `arm-symbian-e32`

This directory is the whole difference between the standard library rustup installs and
the one a `language = "rust-std"` project is built against (design spec §11 step 77,
experiments 89 and 90). It is **not** a fork of `rust-src`: it is an overlay of some
forty files, and the other 82 MB come from the toolchain at build time.

## How a build uses it

A `language = "rust-std"` project builds with plain cargo (experiment 114 §1.4). Its
`.cargo/config.toml` names `[build] rustc = "build/symdev-rustc"`, a link to `symdev`
that runs the pinned `rustc --sysroot <project>/build/sysroot …`. `symdev build` makes that
sysroot in `StdSysroot::materialise` (`crates/symdev-build/src/std_sysroot.rs`):

1. `rustc --print sysroot` names the pinned nightly; its
   `lib/rustlib/src/rust/library` is copied to
   `<project>/build/sysroot/lib/rustlib/src/rust/library` (`StdSrc`).
2. `symbian-rs/crates/symbian-sys` is copied in as `library/symbian-sys`.
3. `overlay/library/` is copied over the top.
4. `lib/rustlib/<host>` is linked to the nightly's own, for the host's crates (proc macros,
   build scripts), and `build/symdev-rustc` to the running `symdev`.

cargo resolves `-Zbuild-std`'s source before any build script runs, so the sysroot must
exist before cargo starts: run `symdev build` once, then `cargo build` as often as you
like. Nothing in the rustup component is touched; it is shared between every project on the
host and rustup overwrites it on the next update.

`StdSrc` writes every file afresh rather than `cp -a`: cargo decides what to rebuild from
modification times, and a copy that kept the source's times could be called unchanged.

## Why a `rustc` wrapper

Experiment 114 §1.4 tried the alternatives. `[env] __CARGO_TESTS_ONLY_SRC_ROOT` in the
config reaches the processes cargo starts, not cargo; `--sysroot` in the target's
`rustflags` is not seen when cargo finds the `build-std` source; a toolchain directory
works only with `librustc_driver` hard-linked (rustc takes its sysroot from that file's
canonical path) and an absolute path or rustup state in the project. A wrapper named by a
config-relative path needs none of that. (0.3.0 ran cargo with
`__CARGO_TESTS_ONLY_SRC_ROOT` itself; that route is gone.)

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
