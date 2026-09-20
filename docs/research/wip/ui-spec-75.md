# WIP: step 75 — design spec for running an Avkon GUI application whose logic is Rust (`docs/research/avkon-rust-spec.md`).

## Findings

- examples/gui LIBRARY line: `euser.lib apparc.lib cone.lib eikcore.lib avkon.lib gdi.lib`; UID `0x100039CE 0xe7351c20`; TARGETTYPE EXE; two START RESOURCE blocks (gui.rss -> \resource\apps with HEADER, gui_reg.rss -> \private\10003a3f\apps).
- The C++ example subclasses exactly four classes: CCoeControl (ConstructL/Draw), CAknAppUi (ConstructL/HandleCommandL/dtor), CAknDocument (CreateAppUiL), CAknApplication (AppDllUid/CreateDocumentL); plus NewApplication() and E32Main().
- Design spec step 75 depends on 70 (shim) and 73 (async/CActiveScheduler); pass criterion is a PID-bound screenshot showing the drawn view and a key press changing it.

- Headers read: eikstart.h (`IMPORT_C static TInt EikStart::RunApplication(TApaApplicationFactory)` — by value; apparc.h has `typedef CApaApplication* (*TFunction)()` and an IMPORT_C implicit ctor `TApaApplicationFactory(TFunction)`).
- Pure virtuals a GUI app must supply, from the headers: CApaApplication::AppDllUid() const, CEikApplication::CreateDocumentL() (PRIVATE pure virtual, no args), CEikDocument::CreateAppUiL(). Everything else on the chain is IMPORT_C with a default (PreDocConstructL, OpenIniFileLC, Capability, CreateDocumentL(CApaProcess*) are implemented by CEikApplication/CAknApplication).
- CAknDocument has only a protected IMPORT_C ctor `CAknDocument(CEikApplication&)` — subclass must forward it.
- CEikAppUi::ConstructL, HandleCommandL(TInt), Exit() are IMPORT_C virtual with defaults; CAknAppUi::BaseConstructL(TInt aAppUiFlags = EStandardApp). CCoeAppUi::HandleKeyEventL(const TKeyEvent&, TEventCode) -> TKeyResponse is IMPORT_C virtual.
- CCoeControl has NO pure virtuals. `Draw(const TRect&) const` is a PRIVATE IMPORT_C virtual; SizeChanged/PositionChanged/FocusChanged are protected virtuals; OfferKeyEventL, CountComponentControls, ComponentControl, HandlePointerEventL are public virtuals.

## Decisions

## Dead ends

## Next step

Read the SDK headers (eikstart.h, aknapp.h, akndoc.h, aknappui.h, coecntrl.h) and nm the DSOs.

(old) Read CLAUDE.md, the Rust SDK design spec (§3, §5, §7, §8, §9, §11), experiments 65a/65, and `examples/gui/`.
