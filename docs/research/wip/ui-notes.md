# WIP: Avkon notes (`symbian-ui::note`)

Task: wrap the Avkon note popups (information / confirmation / warning / error) as
`note::info/confirm/warn/error(&str) -> Result`, via a new `shims/s60/symrs_note.cpp`,
so examples stop using `User::InfoPrint`.

## Findings

- Disassembly route for avkon: the `.dso` export stub at a symbol's address literally
  holds its **ordinal** (`CAknResourceNoteDialog::ExecuteLD(const TDesC16&)` = 1996);
  the ROM `avkon.dll` (`~/.local/share/EKA2L1/data/drives/z/rm-469/sys/bin/`) is a
  `TRomImageHeader` image whose `iExportDir` is at file offset
  `iExportDir - iCodeAddress + 0x78` — **`sizeof(TRomImageHeader)` is 0x78** here,
  found by brute force against Thumb prologues. Entry addresses are odd (Thumb).
  Useful for future SDK questions; it did not settle the leave path cheaply, because
  that needs `CEikDialog::RunLD` across a second DLL.
- `aknnotewrappers.h` declares exactly six classes: `TAknNoteResData`, `CAknNoteWrapper`
  (base, `ExecuteLD(TInt)` / `ExecuteLD(TInt, const TDesC&)`), `CAknResourceNoteDialog`
  (helper base, `ExecuteLD()` / `ExecuteLD(const TDesC&)`), and the four concrete ones:
  `CAknConfirmationNote`, `CAknInformationNote`, `CAknErrorNote`, `CAknWarningNote`.
  Each concrete one has three constructors: `()`, `(TBool aWaitingDialog)`, `(T** aSelfPtr)`.
- All of them are exported from `avkon.dso` (`nm -D --defined-only`), e.g.
  `_ZN19CAknInformationNoteC1Ev` and `_ZN22CAknResourceNoteDialog9ExecuteLDERK7TDesC16`.
  `ExecuteLD(const TDesC&)` lives on `CAknResourceNoteDialog`, not on the concrete class.
- `eikdialg.h` on `CEikDialog::ExecuteLD`/`RunLD`: "The function returns immediately
  unless `EEikDialogFlagWait` has been specified in the `DIALOG` resource." So a plain
  `CAknInformationNote()` (`R_AKN_INFORMATION_NOTE`) should NOT block, and the
  `TBool aWaitingDialog = ETrue` form (`R_AKN_..._NOTE_WAIT`) should. To be measured.
- `ExecuteLD` returns `TInt`: zero unless waiting, else the dismissing button id.

## Decisions

- One shim entry, `symrs_note_show(TInt aKind, const TUint16*, TInt)`, not four: the
  four kinds differ only in which class is `new`ed, the signature is identical, and the
  recorded GCCE argv has no `-ffunction-sections` so four functions in one TU cost the
  same as one anyway.
- No `symrs_note.h`: nothing but the one `.cpp` uses it and the Rust side redeclares
  the entry by hand, as `symbian-ui/src/abi.rs` already does for the host table.
- **Ownership under `TRAP`: the header does not say.** `eikdialg.h` and
  `aknnotewrappers.h` document `ExecuteLD` as "loads, displays, and destroys" and say
  nothing about the leave path. Safe reading chosen: the object has already deleted
  itself on every exit path including a leave, so the shim never deletes after the
  `TRAP`. Being wrong that way leaks one dialog on a path that only fires when the note
  could not be shown; the other way is a double delete.
- A new example `symbian-rs/examples/notes` rather than rewriting `examples/ui`, so
  experiment 86's screenshots stay valid and the size figure is attributable. Costs one
  line in `symbian-rs/Cargo.toml`'s `members`.

## Dead ends

## Next step

- Write `shims/s60/symrs_note.cpp` and `crates/symbian-ui/src/note.rs`.
