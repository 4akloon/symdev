# WIP: experiment 105 — shrink the std-shaped file layer

Task: make `symbian-std/src/fs/**` over `symbian-core/src/fs/**` smaller without changing the
application API (`fs::File::create/open`, `OpenOptions`, `read`, `write`, `metadata`,
`read_dir`, `create_dir_all`, `rename`, `remove_file`, `io::Read/Write/Seek`).
Baseline: `examples/files` 9 341 B vs C++ 6 058 B (1.54x). Measure every non-`std-*` example
before/after; `files`, `locale`, `cleanup`, `time` must pass `symdev test --emulator`.
Keep experiment 98's `read_dir` borrowing from the `CDir`. Stay out of `symbian-ui`,
`shims/s60`, `symbian-macros/src/entry.rs`, `symbian-core/src/des/**`,
`symbian-macros/src/fast_write/**`.

## Findings

- Baseline (main de45e20, `.exe` bytes): alloc 3767, async 18603, atomics 8903, cleanup 4472,
  files 9341, fmt 100031, hello-raw 808, hello 1245, locale 8202, net 10640, notes 12805,
  panic 2010, query 14031, shim 4517, spawnee 3076, time 10256, tls 14114, ui-list 12624,
  ui 11645. Script: `~/.cache/fs-size-agent/measure.sh` (res-base.txt).
- filesdemo.elf fs symbols (nm -S): OpenOptions::open 984, read_dir 420, create_dir_all 392,
  metadata 388, core File::open 228, File::create_new 228, File::read 136, fs::write 152,
  write_all<File> 168, FileServer::connect 84, Name::eq 116, Buf16::push_str 448 (des),
  ErrorKind::of 48. rename + remove_file are inlined into E32Main (2 more with_session copies).
- Mechanism 1: `with_session<T>` is generic and inlined in every caller: flag check/set,
  slot check, connect call, the `Option::insert` drop of the old slot (an RHandleBase::Close),
  flag clear — ~100 B per copy, 6 copies in files.
- Mechanism 2: `path_of` returns `Buf16<256>` by value: memclr 512 + push_str + memcpy 516 at
  every call site (6 sites); the Buf16 is moved once more.
- Mechanism 3: `OpenOptions::open` takes `&self`, so all 5 opening strategies are linked in
  every image; every report-writing example links it through `test_report::finish` ->
  `fs::write` -> `File::create`.

- Candidate 1a (one `Request` enum, one `match` over every call in `Request::on`): files
  9341 -> 8698 (-643), but every example that only writes a report grows +230..+350
  (async +269, atomics +354, cleanup +154, locale +337, shim +315, ...): the `match`
  links every arm (Rename, Entry, GetDir, Open...) into every image that makes any request.
  Enum dispatch defeats dead-code elimination. -> dead end as a shape; the call must be
  passed in (fn pointer / `dyn FnMut`), so only the calls an image uses are linked.

- Candidate 1b (`&mut dyn FnMut` call passed in): report-only images still +185..+330.
  `dyn FnMut`'s vtable holds a `call_once` shim that is a second copy of every closure body
  (File::opened closure 188 B twice), and `Opening` passed through the closure is not
  constant-folded (all three RFile calls linked). -> own trait `Call` (vtable = one fn),
  one closure per opening in an `#[inline]` `File::opened`.
- Candidate 1c: files -431, report-only images -56..+109; shared `request` body 380 B.
  Base cost of 2 inlined calls (create_dir_all + fs::write) in atomics was only ~372 B incl.
  `connect`, so the shared body must be small to win at 2 calls.
- Candidate 1d/1e (commit): the `None` slot overwritten with `ptr::write` (no drop check,
  `*slot =` kept a `RHandleBase::Close` branch even with a `&mut` param), the
  flag checked but not set in `request` (its calls never re-enter), the path built in place
  (no 516-byte memcpy), the directory suffix chosen by the caller (one `push_str` of a
  `&'static str` in the body). `request` = 252 B. Sizes (exe, vs base): alloc 0, async -58,
  atomics +4, cleanup -126, files -484 (8857), fmt -91, hello/hello-raw 0, locale -36,
  net -118, notes -54, panic +39, query +7, shim -26, spawnee +8, time -49, tls -118,
  ui-list -73, ui -12.

- Candidate 3 (commit): `create_dir_all` without the `String` it built to add the trailing
  backslash — `ProcessSession::make_dirs` adds it in the request's stack `TFileName`
  (`Request::of_directory`). Same `KErrOverflow` for a 256-unit path without a separator
  (the push fails instead of the longer string). One heap alloc+free fewer per call. vs c1e:
  async -66, atomics -68, cleanup -80, files -50, fmt -135, locale -42, net -15, notes -67,
  query -68, spawnee -478 (the String was its only heap use: RawVec growth left the image),
  time -59, tls -17, ui-list -84, ui -69; panic/shim/alloc/hello 0.

- Candidate 2 (commit): `File::open/create/create_new` name their `Opening` and `FileMode`
  directly instead of building an `OpenOptions` and deciding at run time. files 8807 ->
  8357 (-450; `OpenOptions::open` no longer linked). Every other example 0: with one
  opening call site LLVM had already constant-folded the builder. Behaviour: same RFile
  call, same mode bits, no append/truncate fix-up in any of the three (as before).

- Candidate 4 (commit): no 552/516-byte moves. `Entry::of` constructs its `TEntry` in its
  own frame (via `Entry::new` it was built in a temporary and `memcpy`'d, 552 B, before
  the request); `rename`'s second path is encoded in the frame that uses it
  (`request::rename`, shared by `ProcessSession::rename` and `FileServer::rename`).
  files 8357 -> 8297 (-60), panic 2049 -> 2017 (-32); others 0. `Request::new` bound is
  `FnMut(*mut RFs, *const TDesC16)` so closures written in the argument need no types
  (a `let` closure with untyped params ICEd clippy: "upvar_tys called before capture
  types are inferred", nightly-2026-09-19).
- panic is +7 vs base (2017 vs 2010): two calls (metadata, read_dir) where the base inlined
  both into E32Main; code symbols are -32 vs base, the exe difference is the two `Call`
  vtables in .rodata and their relocations.

## Decisions

## Dead ends

## Next step

Measure baseline sizes of every example; `nm -S --size-sort` on filesdemo.elf.
