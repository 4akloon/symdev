# 65 — the first Rust E32 image from `symdev build` (2026-09-20)

`hello.exe` (752 bytes) is what `symdev new hello --language rust && cd hello &&
symdev build` wrote to `build/hello.exe` on 2026-09-20: `symbian-rs/examples/hello/src/main.rs`
(the scaffold's `src/main.rs`) compiled by cargo on the pinned nightly for
`symbian-rs/targets/arm-symbian-e32.json` with `-Zbuild-std=core,alloc` into
`build/cargo/arm-symbian-e32/release/libhello.a`, linked with symdev's recorded link line
plus `-u _Z7E32Mainv` (`RustBuild::link_args`), post-linked by the native `symdev-elf2e32`
on the recorded EXE argv (`--uid1=0x1000007a --uid3=0xef9f2cab --fpu=softvfp
--targettype=EXE`). `symdev package && symdev run` put it into EKA2L1, whose log reads
`[Service.Notifier]: Trying to display: Hello from Rust SDK` — the same line as the
hand-built 65a image (`../65a-rusthello/`, 755 bytes) and the C++ control.

Experiment record: `docs/research/experiment-backlog.md` §65.
