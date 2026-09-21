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

## Dead ends

## Next step

- Read the context: CLAUDE.md, overlay README, experiment 89, spec §6a and §11 step 77,
  the overlay's `sys/`, and `symbian-rs/crates/symbian-std/src/net/`.
