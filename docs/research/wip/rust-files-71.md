# rust-files (step 71)

Task: build `symbian-std` (`io`/`fs`/`prelude` in `std` shape) over `RFs`/`RFile`, add the
`E:\symdev\results\<uid3>.json` result protocol with `symdev test --emulator`, and an
`examples/files` that round-trips a file through the harness.

## Findings

## Decisions

## Dead ends

## Next step

- Read the spec (§6a, §7, §9, §11), experiments 68/69/72/77/78, `eka2-concurrency.md`
  and all of `symbian-rs/`.

### 2026-09-20, reading

- `f32file.h` (CRLF + extended ASCII: `grep -a`) declares **no leaving member on `RFs`,
  `RFile`, `RDir` or `TEntry`**. Every `IMPORT_C … L(` in the header is on `CDir`,
  `CDirScan`, `CFileBase`, `CFileMan` or `TOpenFileScan`. So the whole file API is
  shim-free; no new shim source is needed for step 71.
- `efsrv.dso` exports everything the slice needs: `_ZN5RFile4OpenER3RFsRK7TDesC16j`,
  `…6CreateE…`, `…7ReplaceE…`, `_ZN5RFile5CloseEv`, `_ZNK5RFile4ReadER5TDes8`,
  `_ZN5RFile5WriteERK6TDesC8`, `_ZNK5RFile4SizeERi`, `_ZN5RFile7SetSizeEi`,
  `_ZNK5RFile4SeekE5TSeekRi`, `_ZN5RFile5FlushEv`, `_ZN3RFs6DeleteERK7TDesC16`,
  `_ZN3RFs6RenameERK7TDesC16S2_`, `_ZNK3RFs3AttERK7TDesC16Rj`,
  `_ZNK3RFs5EntryERK7TDesC16R6TEntry`, `_ZN6TEntryC1Ev`.
- The **8-bit descriptor type nibbles were never observed** (experiment 69 observed only
  the 16-bit family). Rather than guess them, `PtrC8`/`Ptr8` are built by euser's own
  exported constructors — `_ZN6TPtrC8C1EPKhi` (`TPtrC8(const TUint8*, TInt)`) and
  `_ZN5TPtr8C1EPhii` (`TPtr8(TUint8*, TInt aLength, TInt aMax)`) — called as non-static
  members with `this` as argument 0. Nothing about the header word is assumed.
- Constants from the headers: `KMaskDesLength8 = 0xfffffff`, `KShiftDesType8 = 28`
  (`e32des8.h`); `EFileShareExclusive 0 / ReadersOnly 1 / Any 2 / ReadersOrWriters 3`,
  `EFileStream 0`, `EFileStreamText 0x100`, `EFileRead 0`, `EFileWrite 0x200`,
  `EFileReadAsyncAll 0x400`; `ESeekAddress 0 / Start 1 / Current 2 / End 3`;
  `KEntryAttDir = 0x0010` (`f32file.h`).
- Measured with a compile probe (recorded GCCE argv, `-S`, read the immediates):
  `sizeof(TDesC8) 4`, `sizeof(TDes8) 8`, `sizeof(TPtrC8) 8`, `sizeof(TPtr8) 12`,
  `sizeof(RFile) 8`, `sizeof(TEntry) 552` (`movs r0,#138; lsls r0,r0,#2`),
  `__alignof__(TEntry) 8`, `offsetof(TEntry, iAtt/iSize/iModified/iType/iName)
  = 0/4/8/16/28`. Probe: `scratchpad/files71/layout.cpp`.
- `symbian-sys` gained `des8` and an `efsrv/` module (`rfs`, `rfile`, `entry`);
  `symbian-core` gained `des8` (`PtrC8`/`Ptr8`) and an `fs/` module
  (`server`, `file`, `entry`, `session`). Builds clean for the phone target.
- Host side: `symdev-emulator` gained `json.rs` (a small strict JSON reader, no new
  dependency) and `results.rs` (`EmulatorData` for
  `~/.local/share/EKA2L1/drives/e/...`, `TestReport`, `await_report`) plus
  `Eka2l1Backend::stop` (`kill -9`, only a pid still recognisable as EKA2L1).
  `symdev-cli` gained `symdev test --emulator`. Host gate green.
- **Careful: the drive path.** The skill says installed apps live under
  `~/.local/share/EKA2L1/data/drives/e/`, but `EmulatorData::drive_e` currently builds
  `<root>/drives/e`. Check the real tree before the first emulator run.
