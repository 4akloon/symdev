# WIP: Avkon notes (`symbian-ui::note`)

Task: wrap the Avkon note popups (information / confirmation / warning / error) as
`note::info/confirm/warn/error(&str) -> Result`, via a new `shims/s60/symrs_note.cpp`,
so examples stop using `User::InfoPrint`.

## Findings

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

## Dead ends

## Next step

- Settle whether `ExecuteLD` deletes `this` when it leaves; then write the shim.
