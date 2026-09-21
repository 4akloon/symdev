# WIP: step 76 TLS half — `thread_local!` for symbian-std

Task: implement `symbian_std::thread::LocalKey` + `thread_local!` on Symbian's TLS facility, prove it in `symbian-rs/examples/tls` under `symdev test --emulator`, and answer whether step 77 is unblocked.

## Findings

- **There is no `Dll` class in this SDK.** `grep -rn "class Dll\b|Dll::Tls|Dll::SetTls|Dll::FreeTls"` over the whole of `$EPOCROOT/epoc32/include/` matches exactly one line, and it is a *comment* in `banamedplugins.h:249`. `e32std.h` contains the string `Tls` nowhere. So the spec's premise "`Dll::Tls`/`Dll::SetTls`/`Dll::FreeTls` in `e32std.h`" is wrong for S60 3rd FP2.
- **The only TLS surface is `UserSvr`, in `e32svr.h` lines 39-43**, every one marked `@internalAll` or `@internalComponent`:
  `TInt DllSetTls(TInt aHandle, TAny*)`, `TInt DllSetTls(TInt aHandle, TInt aDllUid, TAny*)`, `TAny* DllTls(TInt aHandle)`, `TAny* DllTls(TInt aHandle, TInt aDllUid)`, `void DllFreeTls(TInt aHandle)`.
- **euser.dso exports exactly those five and no `Dll::*` at all** (`nm -D euser.dso | grep -E "_ZN3Dll|Tls"`): `_ZN7UserSvr9DllSetTlsEiPv`, `_ZN7UserSvr9DllSetTlsEiiPv`, `_ZN7UserSvr6DllTlsEi`, `_ZN7UserSvr6DllTlsEii`, `_ZN7UserSvr10DllFreeTlsEi`. Swept every `.dso` in `epoc32/release/armv5/lib` and every `.lib` in `urel`: no other library defines a TLS entry point.
- Threads already share ONE heap: `spawn` creates the worker with its own heap and its first instruction is `User::SwitchAllocator(creator_heap)` (`symbian-std/src/thread/mod.rs`). So a TLS value allocated on a worker and freed on the main thread is NOT a cross-heap free — there is only one heap in the process once a thread exists.

- **PROBED in EKA2L1 (`examples/tls`, 15 passed).** TLS on this platform is **per-thread, many slots, keyed by an arbitrary `TInt` handle the caller chooses**:
  - untouched handle reads null; `DllSetTls` returns 0 and `DllTls` reads the same pointer back;
  - two handles are two independent slots; a repeat set replaces; `DllFreeTls(h)` empties only `h`;
  - **64 of 64 distinct handles held at once, no set failure** — so it is NOT "one slot per EXE";
  - **a worker thread reads null where the creator stored a value, its own store is visible to itself, and the creator's slot is untouched after the join.** Per-thread, decisively.
  - The uid overloads do **not** interoperate with the one-argument ones in either direction (both cross reads came back `0x0`): a 3-arg set with an explicit uid is invisible to `DllTls(h)`, and a 1-arg set is invisible to `DllTls(h, h)`. Use one pair consistently; the SDK uses the **one-argument** forms, which round-trip.
- Emulator-side mechanism (read for understanding only, EKA2L1 is GPL and never copied in): `svc_register_funcs_v93` maps exec 0x4D `dll_tls(h, uid)`, 0x75 `dll_set_tls(h, uid, ptr)`, 0x76 `dll_free_tls(h)`; the store is `kernel::thread_local_data::tls_slots`, a per-thread `unordered_map<handle, slot>` capped at 10000.
- Threads share ONE heap (`User::SwitchAllocator` in `spawn`), so a TLS value boxed on a worker and dropped elsewhere is not a cross-heap free.

## Decisions

## Dead ends

## Next step

- Read the required context (spec §4/§6a/§11, eka2-concurrency, backlog 80/85, symbian-std/src/thread/).
