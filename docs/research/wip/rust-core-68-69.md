# WIP: experiments 68 (`alloc`) and 69 (`symbian-core`)

Task: give the Rust SDK a `GlobalAlloc` over `User::Alloc`/`Free`/`ReAlloc` (68) and a safe
`symbian-core` with errors and the descriptor family (69), with `examples/alloc` and a
rewritten, `unsafe`-free `examples/hello` proving both in EKA2L1's notifier log.

## Findings

- An example under `symbian-rs/examples/<name>` builds in place: drop the scaffold's
  `symdev.toml` beside its `Cargo.toml` and run `symdev build` there. cargo takes the
  absolute `--target` and `--target-dir build/cargo`, so the workspace membership does not
  get in the way. `examples/hello` reproduced the recorded 752-byte E32.
- `e32err.h` on this SDK: `KErrNone=0` down to `KErrCommsBreak=-48`, contiguous, no gaps.
- Descriptor layout from `e32des16.h`: `TDesC16{TUint iLength}` (type nibble <<28 | length,
  `KShiftDesType16=28`, `KMaskDesLength16=0xfffffff`); `TPtrC16 : TDesC16 {const TUint16* iPtr}`;
  `TDes16 : TDesC16 {TInt iMaxLength}`; `TPtr16 : TDes16 {TUint16* iPtr}`;
  `TBufC16<S> : TDesC16 {TUint16 iBuf[__Align16(S)]}`; `TBuf16<S> : TDes16 {TUint16 iBuf[__Align16(S)]}`;
  `HBufC16 : TDesC16 {TText16 iBuf[1]}`. `__Align16(s)` rounds the unit count up to a
  multiple of `sizeof(TInt)/sizeof(TInt16)` = 2.
- The descriptor *type* enum (`EBufC`/`EPtrC`/…) is in no shipped header; 65a observed
  nibble 0 for `_LIT16` (a `TBufC`-shaped literal). Any other nibble is unobserved.
- `RHeap`/`RAllocator` class declarations are not shipped in this SDK (only the inlines in
  `e32cmn.inl`, which show `RHeap::Align(a) = _ALIGN_UP(a, iAlign)` — alignment is a
  per-heap field, so it must be measured, not read).

- **Heap cell alignment measured on the real euser in EKA2L1** (scratch probe, two runs,
  32 cells): every `User::Alloc` payload address has its low 3 bits clear. Run 1, sizes
  1..64: `P0=007000a0`, `addr & 0x1f` = 00 08 10 18 repeating, stride 0x28 = 40 for every
  size <= 33. Run 2, sizes 36..257: `addr & 0x1f` = 00 08 18 08 18 08 18 00 08 10 18 00 08
  10 18 00 — all multiples of 8. `User::AllocLen` = 36, 44, 68, 100, 132, 260 (always
  4 mod 8) and the strides 0x28/0x30/0x48/0x68/0x88 are always multiples of 8: the cell
  size is rounded to 8 and the 4-byte cell header sits *before* an 8-aligned payload.
  Minimum payload on this ROM is 36 bytes. => alignment guarantee to trust: **8**.
- `User::AllocLen` exists (`00000a4c T _ZN4User8AllocLenEPKv`) and is what measured the
  cell lengths; `User::AllocZ`/`User::AllocSize` are exported too.
- **`mem*` resolution observed** (probe map): the Rust object referenced `__aeabi_memclr4`,
  which pulled `libaprobe.a(compiler_builtins-….rcgu.o)`; `memcpy`, `memset`, `__aeabi_mem*`
  all resolve to **compiler_builtins**, never to euser's `memcpy/memset/memmove/memclr` or
  drtaeabi's `__aeabi_mem*` — the Rust archive sits before the DSOs on the recorded link
  line. No duplicate-definition conflict arises; the DSO copies are simply unused.
- **The cost of pulling that member is the whole crate**: compiler_builtins is one codegen
  unit, so one reference dragged 0x2b338 of `.text` in. Probe ELF 351016 B, E32 101611 B.
  Re-linking the same archive with `--gc-sections` added gives ELF 35520 B.

- **The descriptor type nibbles are now observed**, by a C++ probe run on the real euser in
  EKA2L1 (scaffolded `symdev new` console app, `User::InfoPrint` of the raw header words):
  `_LIT16("abc")` = `0x00000003`, `TBufC16<8>("ab")` = `0x00000002`, `TBuf16<8>("abc")` =
  `0x30000003`, `TPtrC16("abcd")` = `0x10000004`, `TPtr16(p,3,8)` = `0x20000003`,
  `HBufC16::New(8)` = `0x00000000`. So **EBufC = 0, EPtrC = 1, EPtr = 2, EBuf = 3**.
- **Sizes and offsets from the same probe:** `sizeof` TDesC16 4, TPtrC16 8, TDes16 8,
  TPtr16 12, TBufC16<8> 20, TBuf16<8> 24, TBufC16<7> 20, TBuf16<7> 24, HBufC16 8.
  Data offsets: TBufC16 +4, TBuf16 +8, HBufC16 +4. `TPtrC16` word 1 is the data pointer;
  `TPtr16` word 1 is `iMaxLength` and word 2 the pointer; `TBuf16` word 1 is `iMaxLength`.
  `User::AllocLen(HBufC16::New(8))` = 36, the heap minimum again.

## Decisions

- `symbian-alloc` holds the allocator *type* only; the single `#[global_allocator]` and
  the `#[alloc_error_handler]` live in `symbian-runtime`, which every application links,
  so an app cannot forget the heap and two libraries cannot install two.
- Over-alignment is **padded, not refused**: `MAX_TRUSTED_ALIGN = 8` (measured). A larger
  `Layout::align` allocates `size + align`, places the payload on the next aligned address
  above the cell and writes the cell address in the four bytes below it; `dealloc` and
  `realloc` branch on `layout.align()` and give euser back the cell it handed out.
  Verified in the emulator: `Box<#[repr(align(32))] …>` came back 32-aligned with its
  bytes intact.
- OOM: `alloc` still returns null (so `try_reserve` works), and the infallible path goes
  through `#[alloc_error_handler]` to `User::Exit(KErrNoMemory)` = `-4`, never a Rust
  panic. `alloc_zeroed` uses `User::AllocZ`.
- `realloc` uses `User::ReAlloc(cell, size, 0)` for normally aligned blocks; the
  over-aligned path re-places by hand because `ReAlloc` may move the cell.
- `--gc-sections` is added to the **Rust** link line only (`RustBuild::link_args`). The
  recorded C++ line stays byte-identical.
- `Des16<'a>` is `&'a dyn DesC16`, not a raw pointer pair, which keeps all of
  `symbian-core`'s `unsafe` inside `hbuf16.rs`. `PtrC16` keeps the borrowed slice after
  its two C-visible words for the same reason.
- `Buf16<N>` is the observed `TBuf16<N>` (`EBuf`, `iMaxLength`, data at +8) with an array
  of exactly `N` units; C++ rounds an odd `N` up one unit (`__Align16`), which changes
  only `sizeof`, and `iMaxLength = N` keeps every euser write inside the array.
- `EBufCPtr` (`RBuf16`) has no Rust type: not observed.


## Dead ends

### Results

- **Experiment 68 passes.** `examples/alloc` in EKA2L1:
  `Trying to display: heap 0,1,4,9,16,` and
  `Trying to display: alloc sum=85344 cap=64 a8=0 a32=0 byte=a5 heap=16`.
  `sum(i^2, i=0..63) = 85344` proves a `Vec<u16>` survived every `User::ReAlloc` growth;
  `a8=0` the 8-alignment; `a32=0` and `byte=a5` the padded over-aligned `Box`; `heap=16`
  an `HBuf16` used as a `const TDesC16&`.
- **Experiment 69 passes.** `examples/hello`, no `unsafe` anywhere:
  `Trying to display: Hello from Rust SDK (19 chars)`.
  `examples/hello-raw` (the old raw version) still prints `Hello from Rust SDK`.
- **E32 sizes** (`--gc-sections` on): `hello-raw` **752 B** — byte-for-byte the recorded
  experiment-65 size, so the flag changes nothing for a program that pulls no
  `compiler_builtins`; `hello` (safe, `write!`) **10 375 B**, and **8 086 B** with
  `push_str` instead of `write!`, so `core::fmt` costs ~2.3 kB and the descriptors and
  the heap ~7.3 kB; `alloc` **11 499 B** (**104 560 B** without `--gc-sections`).
- The residual ~7 kB is `compiler_builtins`' globally visible helpers: in a `-shared`
  link every dynamic symbol is a garbage-collection root, so `--gc-sections` cannot drop
  them. Making archive symbols local (`--exclude-libs`) is the next thing to try; not
  done here.

## Next step

- Read the design spec §5–§7, §11 and backlog 65/65a; then measure `User::Alloc` alignment.
