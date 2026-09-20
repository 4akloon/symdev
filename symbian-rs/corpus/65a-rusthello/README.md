# 65a — the first Rust E32 image (2026-09-20)

`hello.rs` compiled with stable rustc 1.98.1 for the stand-in target
`armv5te-unknown-linux-gnueabi` (`--crate-type staticlib -C panic=abort
-C relocation-model=static -C opt-level=s -C force-unwind-tables=no`), linked with symdev's
recorded link line plus `-u _Z7E32Mainv`, post-linked by the native `symdev-elf2e32` on the
recorded EXE argv (`--uid1=0x1000007a --uid3=0xe0000065 --fpu=softvfp --targettype=EXE`).
`rusthello.exe` is 755 bytes. In EKA2L1 it reached the notifier service with
"Hello from Rust SDK" — the same log lines as the C++ control in experiment 65a.
`lit.cpp` is the probe whose compiled bytes fixed the `_LIT` descriptor layout.

Experiment record: `docs/research/experiment-backlog.md` §65a.
