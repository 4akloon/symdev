# WIP: step 75 — design spec for running an Avkon GUI application whose logic is Rust (`docs/research/avkon-rust-spec.md`).

## Findings

- examples/gui LIBRARY line: `euser.lib apparc.lib cone.lib eikcore.lib avkon.lib gdi.lib`; UID `0x100039CE 0xe7351c20`; TARGETTYPE EXE; two START RESOURCE blocks (gui.rss -> \resource\apps with HEADER, gui_reg.rss -> \private\10003a3f\apps).
- The C++ example subclasses exactly four classes: CCoeControl (ConstructL/Draw), CAknAppUi (ConstructL/HandleCommandL/dtor), CAknDocument (CreateAppUiL), CAknApplication (AppDllUid/CreateDocumentL); plus NewApplication() and E32Main().
- Design spec step 75 depends on 70 (shim) and 73 (async/CActiveScheduler); pass criterion is a PID-bound screenshot showing the drawn view and a key press changing it.

## Decisions

## Dead ends

## Next step

Read the SDK headers (eikstart.h, aknapp.h, akndoc.h, aknappui.h, coecntrl.h) and nm the DSOs.

(old) Read CLAUDE.md, the Rust SDK design spec (§3, §5, §7, §8, §9, §11), experiments 65a/65, and `examples/gui/`.
