# Experiment 65 (WIP): `symdev new --language rust` → build → package → run on EKA2L1

Task: make the hand-built 65a Rust `E32Main` a product path — `symdev new hello --language rust && symdev build && symdev package && symdev run` prints the `[Service.Notifier]: Trying to display: Hello from Rust SDK` line in `build/eka2l1.log` with no hand steps.

## Findings

## Decisions

## Dead ends

## Next step

- Step 1: create `symbian-rs/` workspace, `rust-toolchain.toml`, derive `targets/arm-symbian-e32.json`.
- 2026-09-20 nightly on host: `rustc 1.100.0-nightly (feaadeeac 2026-09-19)`, components cargo, rust-src, rust-std (host only). Pin `channel = "nightly-2026-09-19"`.
- Derived spec (`rustc +nightly -Z unstable-options --print target-spec-json --target armv5te-none-eabi`): abi=eabi, arch=arm, asm-args [-mthumb-interwork -march=armv5te -mlittle-endian], atomic-cas=false, c-enum-min-bits=8, data-layout `e-m:e-p:32:32-Fi8-i64:64-v128:64:128-a:0:32-n32-S64`, emit-debug-gdb-scripts=false, features `+soft-float,+strict-align`, frame-pointer=always, has-thumb-interworking=true, linker=rust-lld/gnu-lld, llvm-floatabi=soft, llvm-target=armv5te-none-eabi, max-atomic-width=0, panic-strategy=abort, relocation-model=static, pointer width 32. So the spec's panic/reloc/atomic/gdb rows are already the compiler's defaults, no override needed.
- Spec disagreement: `c-enum-min-bits` is 8 in the derived JSON (AAPCS bare-metal short enums), spec guessed 32 — decide by the C++ probe below.
- euser.dso mangled names (nm -D): `_ZN4User4ExitEi`, `_ZN4User4FreeEPv`, `_ZN4User5AfterE27TTimeIntervalMicroSeconds32`, `_ZN4User5AllocEi`, `_ZN4User5PanicERK7TDesC16i`, `_ZN4User7ReAllocEPvii`, `_ZN4User9InfoPrintERK7TDesC16`.
- Host CLI has `--lang` (not `--language`) and a required `--target`; the one-liner needs `--language rust` and no `--target` → add the alias and default `--target nokia-e52` (only value).
- Root `Cargo.toml` is a workspace: a nested `symbian-rs/` workspace needs `exclude = ["symbian-rs"]` at the root or cargo refuses.
- **Enum probe (spec §10 item 8) settled:** `probe.cpp` (`enum TSmall {EA,EB}`, `TCapability`) compiled with the observed GCCE argv gives `sizeof == 4` for both → GCCE does not short-enum; `c-enum-min-bits = 32` overrides the derived 8. Spec §4 row was right, the compiler's bare-metal default was wrong.
- `memcpy`/`memset`/`memmove`/`memclr` are versioned exports of `euser.dso`; `__aeabi_mem*` of `drtaeabi.dso`; neither `libgcc.a` nor `usrt2_2.lib`/`eexe.lib` define them. `memcmp` is exported by nobody on the link line. Decision: no `compiler-builtins-mem` feature; Rust's `memcpy`/`memset` bind to euser like C++'s do; `memcmp`/`bcmp` is a TODO if a program ever needs it (check the map).
- Decision: `symbian_runtime::entry!(main)` macro expands to the `_Z7E32Mainv` export in the app crate (a plain `fn main()` in a `no_main` staticlib is private; the runtime cannot name it without a symbol contract). Panic handler lives in symbian-runtime.
- Decision: scaffold writes absolute paths (SDK crates + target JSON) from `SYMDEV_RUST_SDK` or the compiled-in `<checkout>/symbian-rs`; RustBuild passes `--release --target <json> -Zbuild-std=core,alloc` explicitly and the scaffolded `.cargo/config.toml` repeats the two settings so a hand `cargo build` matches. Stopgap until an installed SDK layout exists.
- `rust-toolchain.toml` `channel = "nightly-2026-09-19"` made rustup auto-install that dated build (1.100.0-nightly 420ed2a0c 2026-09-18); the floating `nightly` on the host is one day newer (feaadeeac 2026-09-19 = nightly-2026-09-20). Kept the dated pin: exact, now on disk.
- Cargo (this nightly) refuses `.json` target specs unless `-Zjson-target-spec` is given; `[unstable] json-target-spec = true` in `.cargo/config.toml` works. symdev must pass it too.
- A crate whose lib is `src/main.rs` needs `autobins = false`, else cargo also makes a `bin` target and the target JSON (`executables: false`) refuses it.
- `cargo build --release` with build-std: `libhello.a` = 902900 bytes, two members: the LTO'd `hello…cgu.0.rcgu.o` (only `U _ZN4User5After…`, `U _ZN4User9InfoPrint…`) and `compiler_builtins…rcgu.o` (all of libm, `__aeabi_*`, and `mem` symbols). Both carry `.ARM.exidx.*` sections although panic=abort and `default-uwtable=false` (spec §4 item 1: rustc does emit them) — the link/post-link decides whether that matters.
- Hand link of the custom-target `libhello.a` with the recorded argv + `-u _Z7E32Mainv` (after `-u _E32Startup`): rc 0, ELF 15228 bytes, 6 NEEDED, only `libhello.a(hello…cgu.0.rcgu.o)` pulled — the `compiler_builtins` member (all symbols weak `W`) is not pulled, so spec §4 item 4 (collisions with `-lgcc`/euser `memcpy`) does not arise for hello; still open for a program that references `memcpy`/`__aeabi_*` (weak vs euser's strong versioned export — the archive comes first on the line, so ld would pull the weak member; to be observed in 66).
- `.ARM.exidx`/`.ARM.extab` are present in the output ELF (from eexe/usrt objects as for C++, plus hello's one CANTUNWIND entry); native `elf2e32` accepted the ELF: E32 **752 bytes** (65a: 755, stand-in target, name `rusthello`). `file`: Psion Series 5 executable.
- **Step 3 passed by hand (2026-09-20):** custom-target build → recorded link → native elf2e32 (752 B) → `symdev package` (bld.inf-less project, uid3 0xe0000065) → `symdev run`: `eka2l1.log` line 377 `[Service.Notifier]: Trying to display: Hello from Rust SDK`, line 378 the known `Corrupted graphics command list! Emulation halt.`; the emulator process exited by itself. Same two lines as 65a and its C++ control.
