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
- **Layouts measured** with a compile probe on the recorded GCCE argv
  (`scratchpad/net74/probe.cpp`, `objdump -d`; note `movs rN,#k; lsls rN,#2` = k*4):
  `sizeof` `TSockAddr` **40** (align 4), `TInetAddr` **40** (align 4, adds no members),
  `RSocketServ` **4**, `RSocket` **8**, `RHostResolver` **8**, `TRequestStatus` **8**
  (align 4), `TNameRecord` **564**, `TNameEntry` (= `TPckgBuf<TNameRecord>`) **576**
  (align 8), `TSockXfrLength` (= `TPckgBuf<TInt>`) **16**; `TNameRecord` offsets
  `iName` 0, `iAddr` 520, `iFlags` 560.
- **`es_sock.h` needs the case-fold overlay**: it reaches `MetaData.h`, `Metadata.inl`
  and `MetaContainer.inl`, which are `metadata.h`/`metadata.inl`/`metacontainer.inl` on
  disk. Three links; nothing else in the two headers needs one.
- **`TNameEntry` can be built from euser's own exported constructor.**
  `TPckgBuf<T>()` is `TAlignedBuf8<sizeof(T)>(sizeof(T))` (`e32cmn.inl:2659`, `:1179`),
  which is `TBufBase8(aLength, S)` — and `TBufBase8(TInt,TInt)` is exported as
  `_ZN9TBufBase8C1Eii`. So 576 zeroed 8-aligned bytes plus that one call is a valid
  `TNameEntry` with no header word guessed, exactly as `PtrC8`/`Ptr8` are built.
- Constants from the headers: `KAfInet` 0x0800, `KAfInet6` 0x0806, `KAFUnspec` 0,
  `KSockStream` 1, `KSockDatagram` 2, `KProtocolInetTcp` 6, `KProtocolInetUdp` 17,
  `KInetAddrLoop` 127.0.0.1, `THostName` = `TBuf<0x100>`; `RSocket::TShutdown` is
  `ENormal 0, EStopInput 1, EStopOutput 2, EImmediate 3`.
- **EKA2L1 registers the TCP and UDP inet protocols unconditionally**
  (`internet/protocols/overall.cpp:44`, both `INET_TCP_PROTOCOL_ID` and
  `INET_UDP_PROTOCOL_ID`, families `{INET_ADDRESS_FAMILY, INET6_ADDRESS_FAMILY}`), over
  libuv on the host; no config flag to turn on.

- **End to end in the emulator on the first run** (`examples/net` smoke cut, notifier
  line): `net74 session=0 resolver=0 lookup=2130706433 tcp=0 connect=0 send=0 recv=10
  b0=80 alive`. So `RSocketServ::Connect`, `RHostResolver::Open`, the synchronous
  `GetByName` into a hand-built `TNameEntry` (2130706433 = 0x7f000001 = 127.0.0.1 for
  `localhost`), `RSocket::Open`, `Connect`, `Send` and `RecvOneOrMore` all work, and the
  `TRequestStatus` + `User::WaitForRequest` pair is enough — **no executor**.
- **The guest reaches the host's loopback directly.** A Python server bound to
  `0.0.0.0:18974` on this host saw `connection from ('127.0.0.1', …)` from the emulated
  app connecting to `127.0.0.1:18974`. No port mapping, no special address.
- `examples/net` E32 at the smoke cut: **2 646** bytes (no `core::fmt`, no files).

## Decisions

- **No C++ shim for this step.** Nothing on the path leaves and nothing is sret, so
  `symbian-sys` declares esock/insock/euser directly. `shims/common/symrs_esock.cpp` is
  not created.
- Layering mirrors step 71's files: `symbian-sys::esock` (raw) →
  `symbian-core::net` (safe, owns the `unsafe`) → `symbian_std::net` (`std`'s shape,
  `#![forbid(unsafe_code)]`).
- `TRequestStatus` and `User::WaitForRequest` are euser's, but live in
  `symbian-sys/src/esock/request.rs` for now: three other agents are in this tree and
  `euser.rs` is the likeliest shared file. Move them when a second subsystem needs them.
- IPv4 only. `core::net::Ipv6Addr` is refused with `TODO: … (not observed)` rather than
  guessed, because `TInetAddr`'s v6 path was never exercised here.

## Dead ends

## Next step

- Read the design spec §6a/§7/§11, experiments 78 and 79, `symbian-std/src/{io,fs}` and
  `shims/common/symrs_shim.h`.
