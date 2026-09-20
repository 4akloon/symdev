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

## Decisions

## Dead ends

## Next step

- Read the design spec §5–§7, §11 and backlog 65/65a; then measure `User::Alloc` alignment.
