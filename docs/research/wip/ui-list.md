# WIP: S60 list box for symbian-ui (branch `ui-list`)

Task: add an S60 Avkon list box to the symdev Rust SDK — `symbian-ui/src/list.rs` +
`shims/s60/symrs_list.cpp` — with a safe `List::new/set_items/selected/on_select`
surface, proven in the emulator with arrow keys and Return.

## Findings

- Spec §3: ABI is shape B — two `#[repr(C)]` tables, `SymRsHost` (down) and `SymRsAppVtbl`
  (up), each with a leading `u32 size`; single exported symbol `symrs_app_vtbl()`.
- Spec §4.3: every host entry that can leave is `TRAP`ped **inside the shim**; Rust never
  has a frame on the stack when a leave starts. `Draw` is called OUTSIDE a trap harness,
  is `const`, and nothing reachable from it may leave — by construction, not by TRAP.
- Spec §5.2: keys — `EKeyUpArrow` 0xf809, `EKeyDownArrow` 0xf80a, `EKeyDevice3` (select)
  0xf845; `EEventKey = 1`; `EKeyWasNotConsumed = 0` / `EKeyWasConsumed = 1`.
- exp 83 / eka2l1-input.md: XTEST is dropped on this host; `XSendEvent` to the emulator
  TOPLEVEL works. Driver is `docs/research/acceptance/emukey.py {keys,shot,focus} <pid>`.
  Arrows + Return verified; **F1/F2 softkeys are dead in our own apps — do not test them.**
- exp 86: existing view is stacked with `SetMopParent(this); ConstructL(ClientRect());
  AddToStackL(iView)`. Client area measured 240x245. `uidemo.exe` = 12 715 bytes.
- exp 86: link needs `-u symrs_app_vtbl`; `-l:euser.dso -l:drtaeabi.dso` must stay BEFORE
  the Rust archive (that ordering is the whole reason the E32 is 12 KB and not 107 KB).

## Decisions

## Dead ends

## Next step

- Read the spec (§3 forwarding ABI, §4 control/key contracts, §5 per-virtual leave rule),
  `eka2l1-input.md`, experiments 83 and 86, then `symbian-ui/` and `symrs_avkon.cpp` 198-200.
