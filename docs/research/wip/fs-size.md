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

## Decisions

## Dead ends

## Next step

Measure baseline sizes of every example; `nm -S --size-sort` on filesdemo.elf.
