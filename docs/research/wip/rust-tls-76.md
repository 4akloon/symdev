# WIP: step 76 TLS half — `thread_local!` for symbian-std

Task: implement `symbian_std::thread::LocalKey` + `thread_local!` on Symbian's TLS facility, prove it in `symbian-rs/examples/tls` under `symdev test --emulator`, and answer whether step 77 is unblocked.

## Findings

- **There is no `Dll` class in this SDK.** `grep -rn "class Dll\b|Dll::Tls|Dll::SetTls|Dll::FreeTls"` over the whole of `$EPOCROOT/epoc32/include/` matches exactly one line, and it is a *comment* in `banamedplugins.h:249`. `e32std.h` contains the string `Tls` nowhere. So the spec's premise "`Dll::Tls`/`Dll::SetTls`/`Dll::FreeTls` in `e32std.h`" is wrong for S60 3rd FP2.
- **The only TLS surface is `UserSvr`, in `e32svr.h` lines 39-43**, every one marked `@internalAll` or `@internalComponent`:
  `TInt DllSetTls(TInt aHandle, TAny*)`, `TInt DllSetTls(TInt aHandle, TInt aDllUid, TAny*)`, `TAny* DllTls(TInt aHandle)`, `TAny* DllTls(TInt aHandle, TInt aDllUid)`, `void DllFreeTls(TInt aHandle)`.
- **euser.dso exports exactly those five and no `Dll::*` at all** (`nm -D euser.dso | grep -E "_ZN3Dll|Tls"`): `_ZN7UserSvr9DllSetTlsEiPv`, `_ZN7UserSvr9DllSetTlsEiiPv`, `_ZN7UserSvr6DllTlsEi`, `_ZN7UserSvr6DllTlsEii`, `_ZN7UserSvr10DllFreeTlsEi`. Swept every `.dso` in `epoc32/release/armv5/lib` and every `.lib` in `urel`: no other library defines a TLS entry point.
- Threads already share ONE heap: `spawn` creates the worker with its own heap and its first instruction is `User::SwitchAllocator(creator_heap)` (`symbian-std/src/thread/mod.rs`). So a TLS value allocated on a worker and freed on the main thread is NOT a cross-heap free — there is only one heap in the process once a thread exists.

## Decisions

## Dead ends

## Next step

- Read the required context (spec §4/§6a/§11, eka2-concurrency, backlog 80/85, symbian-std/src/thread/).
