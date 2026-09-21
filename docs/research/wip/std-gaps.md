# WIP: closing the gaps left by step 77 in `std` for `target_os = "symbian"`

Task: implement `sys/path` drive prefixes, `read_dir` + `args`, a real `sys/net` backend
and a partial `process` backend in the `symbian-rs/rust-src` overlay, keep `env`
`Unsupported`, and prove it all through `symdev test --emulator`.

## Findings

- `f32file.h` line ~1648 (RFs class doc, point 5): "maintaining a default path; **unlike
  some other systems, there is a single system default path, rather than one for each
  drive**: the default path consists of a drive and a path specification." So Symbian has
  **no per-drive current directory**; `RFs::SessionPath`/`SetSessionPath` (f32file.h 1733-4)
  are the one per-session default. `C:x` is therefore still relative (it needs the session
  path's directory), so `Prefix::Disk` + `is_absolute() == has_root() && prefix.is_some()`
  is the right shape, but for a different reason than Windows'.
- `RFs::GetDir` has three overloads (f32file.h 1740-1742), all `IMPORT_C TInt`, i.e.
  non-leaving at the RFs level; the leaving ones are the private `GetDirL`/`DoGetDirL`
  (1827-1830). `CDir` itself is a `CArrayPakFlat` whose accessors may leave.
- The `grep` in this shell is a wrapper honouring ignore files and returns nothing for the
  SDK tree; use `/usr/bin/grep` for anything under `$SYMDEV_EPOCROOT`.

- `CDir`'s public interface is `IMPORT_C virtual ~CDir()`, `Count() const`, `operator[](TInt) const`,
  `Sort(TUint)`; `NewL`/`AddL`/`ExtractL` are **protected** (friend `RFs`). `RFs::GetDir`
  returns `TInt`, so the leave is trapped inside efsrv, not by us. What *does* need the
  shim is `delete aDir` — the destructor is virtual (shim rule 3).
- Exports confirmed in `efsrv.dso`: `_ZNK3RFs6GetDirERK7TDesC16jjRP4CDir`,
  `_ZNK4CDir5CountEv`, `_ZNK4CDirixEi`, `_ZN4CDirD0Ev`, `_ZNK3RFs11SessionPathER6TDes16`.
- `TEntry` offsets were already measured in experiment 79 and are written in
  `symbian-sys/src/efsrv/entry.rs`: `iAtt 0`, `iSize 4`, `iModified 8`, `iType 16`,
  **`iName 28`** — so a directory entry's name needs no new measurement.
- **There is no `RProcess::CommandLine`.** The SDK's exports are
  `_ZN4User11CommandLineER6TDes16` and `_ZN4User17CommandLineLengthEv` (`e32std.h`
  4569-4570, class `User`). One `TDes16`, one string, no vector and no quoting rule.
- `RProcess::Create(const TDesC&, const TDesC&, TOwnerType)`
  (`_ZN8RProcess6CreateERK7TDesC16S2_10TOwnerType`), `Resume`, `Logon`, `ExitType`,
  `ExitReason`, `Kill`, `Terminate` are all exported and none leaves.
- **`RPipe` does not exist in this SDK**: no header under `epoc32/include` declares it
  (it is 9.4+). Verified, not taken on trust. So `Stdio::piped()` is `Unsupported`.
- `es_sock.h` has no `RSocketServ::ShareAuto`, so a socket-server session cannot be
  shared between threads on observed evidence; `sys::net` gets a **per-thread**
  `RSocketServ`, exactly as `sys::fs` has a per-thread `RFs`.

## Decisions

- **Step 1 done.** `sys/path/symbian/` (mod.rs + a pure `drive.rs`) replaces
  `unsupported_backslash` for this target. `Prefix::Disk`, uppercased; no UNC, no
  verbatim, no device namespace — Symbian has none. `absolute` stays `unsupported()`
  with `TODO: RFs::Parse / TParse (not observed)`.
- The host test lives in `crates/symdev-build/src/std_src/tests.rs` and compiles the
  overlay's own `drive.rs` through `#[path]`, so the tested file *is* the shipped file.
  (`include!` does not work: the file's `//!` docs are inner attributes and a macro
  cannot expand to those.)
- Emulator: `stdhello: 30 passed`, `symdev test --emulator` exit 0; E32 52 908 bytes
  (was 52 210, so the prefix parser costs 698).

- **Step 2 done.** `read_dir` over `RFs::GetDir` + `CDir` (`sys/fs/symbian/dir.rs`);
  `args` over `User::CommandLine` + `RProcess().FileName()` for element 0
  (`sys/args/symbian.rs`); `env` a deliberate empty-but-not-panicking backend
  (`sys/env/symbian.rs`). `stdhello: 41 passed`, E32 67 303 bytes.
- **The real blocker was not a leave: no thread had a `CTrapCleanup`.** `RFs::GetDir`
  uses the cleanup stack inside its own TRAP, and a thread with no trap handler panics
  `E32USER-CBase 69` (`EClnNoTrapHandlerInstalled`) — which the emulator logs only at
  `Kernel:trace`, so at the default `log-filter` it looks exactly like experiment 76's
  silent death. `std` now installs one in `rt::symbian_start` and in `sys::thread`'s
  trampoline, which is what a Symbian `E32Main` and every thread it creates conventionally
  do. **The `no_std` runtime still installs none** — out of scope here, worth a step.
- `RFs::GetDir` does **not** need a TRAP of its own: it returns `TInt` and traps
  `GetDirL` itself (proved by putting a shim TRAP around it, which changed nothing, and
  then by the emulator's own "Leave trapped by trap handler" for the missing-directory
  case). The only shim `read_dir` needs is `delete aDir`, because `~CDir` is virtual.
- Measured with a compile probe on the recorded GCCE argv: `sizeof(TPtr16)` **12**,
  align 4; `sizeof(RProcess)` **4**; `sizeof(TBuf16<256>)` **520**;
  `sizeof(TRequestStatus)` **8** (confirming experiment 84).
- **EKA2L1 quirk, not the file server's:** its `mkdir` answers `KErrAlreadyExists` when
  the *parent* is missing, where Symbian answers `KErrPathNotFound`. `create_dir_all`
  reads that as "already there" and stops, so more than one missing level silently
  creates nothing. One level at a time works.
- **EKA2L1 quirk:** a file's size reads back as 0 from both `RFs::Entry` and a directory
  listing until `RFile::Flush`; closing the handle is not enough.
- One wrapper per shim translation unit: the recorded GCCE argv has no
  `-ffunction-sections`, so `--gc-sections` drops an unused wrapper only when it is the
  whole of an object's text. Sharing `symrs_f32.cpp` cost `examples/shim` 26 bytes;
  splitting brought every `no_std` example back to its recorded size to the byte.

- **Step 3 done.** `sys/net/connection/symbian/` (session, request, addr, socket, tcp,
  udp, mod) is step 74's blocking layer re-hosted; `examples/std-net` reports
  **31 passed** through `symdev test --emulator`, E32 53 374 bytes.
- The step-73 refusal is verified, not assumed: the example installs a
  `CActiveScheduler` through the shim and `TcpStream::connect` answers `ResourceBusy`
  (`KErrInUse`) instead of hanging.
- `io::Error::other(..)` gives `ErrorKind::Other`, not `Unsupported` — four refusal
  cases failed until `no_option` was changed to `io::Error::new(Unsupported, ..)`.

- **Step 4 done.** `sys/process/symbian/` (mod + status): `Command::spawn`, `wait`,
  `try_wait`, `kill`, `ExitStatus`, `getpid` over `RProcess`; `Stdio::piped()`,
  `output()`, `current_dir` and any env change are refused at the spawn.
  `std::process::exit` is now `User::Exit` (a `sys/exit.rs` replacement) — the default
  arm was `intrinsics::abort()`, an undefined instruction that loses the code.
  `stdhello: 50 passed`, E32 73 633 bytes.
- `examples/spawnee` (3 208 bytes, `no_std`) is the child std-hello spawns, and it has
  to be a separate image: **EKA2L1 cannot spawn an image with a writable data section
  through the loader.** `RProcess::Create` succeeds, the emulator gives the child an
  extra `anonymous` 0x1000-byte chunk at 0x400000 for its data, and the child dies with
  `KERN-EXEC 3` reading its heap base + 0xA4 before `main`. Every `std` image tried does
  this; no `no_std` one does (their `runtime data` is logged as `0x0`). The parent then
  hangs on a `Logon` that never completes.
- `RProcess::Id()` returns an 8-byte `TProcessId`, which the EABI returns indirectly, so
  `std::process::id` goes through a shim (`symrs_process_id`) as `FileName()` does.

## Dead ends

## Next step

- Read the context: CLAUDE.md, overlay README, experiment 89, spec §6a and §11 step 77,
  the overlay's `sys/`, and `symbian-rs/crates/symbian-std/src/net/`.
