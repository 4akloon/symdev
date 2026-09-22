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

## Decisions

## Dead ends

## Next step

Measure baseline sizes of every example; `nm -S --size-sort` on filesdemo.elf.
