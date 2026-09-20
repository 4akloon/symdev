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

## Dead ends

## Next step

- Read the design spec §5–§7, §11 and backlog 65/65a; then measure `User::Alloc` alignment.
