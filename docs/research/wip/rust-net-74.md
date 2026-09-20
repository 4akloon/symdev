# rust-net-74

Step 74 of the Rust SDK: `symbian_std::net` — blocking TCP/UDP/resolver in `std`'s shape over
`RSocketServ`/`RSocket`/`RHostResolver`, no async runtime.

## Findings

- **Nothing in the socket API leaves.** `es_sock.h` declares no leaving member on
  `RSocketServ`, `RSocket` or `RHostResolver` — every `IMPORT_C … L(` in that header is on
  the `CSubCon*` parameter-bundle classes (lines 1204-1394). `in_sock.h` has **no** leaving
  member at all. So, like `f32file.h` in step 71, this step is shim-free for esock/insock.
- The one non-C signature on the path: `RSocketServ::Version()` returns `TVersion` by value
  (sret) — not needed, so not declared.
- `RHostResolver::GetByName` has a **synchronous** overload
  (`_ZN13RHostResolver9GetByNameERK7TDesC16R8TPckgBufI11TNameRecordE`, returns `TInt`), so
  resolution needs no `TRequestStatus` at all.
- `User::WaitForRequest(TRequestStatus&)` = `_ZN4User14WaitForRequestER14TRequestStatus`
  in euser.dso.
- `TUint32` mangles as `m` (`unsigned long`): `TInetAddr(TUint32,TUint)` is
  `_ZN9TInetAddrC1Emj`, `SetAddress(TUint32)` is `_ZN9TInetAddr10SetAddressEm`.
- `TSockAddr : public TBuf8<KMaxSockAddrSize>` with `KMaxSockAddrSize = 0x20`
  (`es_sock.h:193`); `TInetAddr : public TSockAddr`. Sizes to be measured, not assumed.

## Decisions

## Dead ends

## Next step

- Read the design spec §6a/§7/§11, experiments 78 and 79, `symbian-std/src/{io,fs}` and
  `shims/common/symrs_shim.h`.
