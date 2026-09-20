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

- Key codes probed (host g++ over `e32keys.h`, values are header constants): EKeyLeftArrow 0xf807, Right 0xf808, Up 0xf809, Down 0xf80a, EKeyDevice0 0xf842 (LSK), EKeyDevice1 0xf843 (RSK), EKeyDevice3 0xf845 (selection), EKeyYes 0xf862, EKeyNo 0xf863, EKeyMenu 0xf836; scan codes EStdKeyDevice0 0xa4, Device1 0xa5, Device3 0xa7, arrows 0x0e/0x0f/0x10/0x11, EStdKeyYes 0xc4, EStdKeyNo 0xc5. TKeyEvent = { TUint iCode; TInt iScanCode; TUint iModifiers; TInt iRepeats; } (16 bytes, POD). TKeyResponse { EKeyWasNotConsumed=0, EKeyWasConsumed=1 }. TEventCode: EEventNull=0, EEventKey=1, EEventKeyUp=2, EEventKeyDown=3.
- Built examples/gui with the worktree's symdev: gui.o has **182 undefined symbols** for four subclasses (every inherited virtual the emitted vtables reference), incl. `__gxx_personality_v0`, `__cxa_end_cleanup`, `__aeabi_unwind_cpp_pr0`.
- Import split in gui.elf: cone 67, eikcore 58, avkon 38, drtaeabi 19, euser 11, apparc 6, scppnwdl 1, gdi 1. `EikStart::RunApplication` -> eikcore; `TApaApplicationFactory(TFunction)` ctor -> apparc; `CFont::AscentInPixels` -> gdi (the only gdi import); `CEikonEnv::TitleFont` -> eikcore; `CCoeEnv::Static` -> cone.
- **Drawing needs no extra library:** every `CWindowGc` call in the example (Clear, SetPenColor, UseFont, DrawText, DiscardFont) is a pure virtual of `CGraphicsContext`, dispatched through the vtable — ws32.lib is absent from the LIBRARY line and ws32 absent from NEEDED.
- The observed GCCE compile argv passes `-mthumb` and `-mthumb-interwork` (`crates/symdev-build/src/driver/compile.rs`), so shim C++ is **Thumb** while rustc on `arm-symbian-e32` emits **ARM** — interworking across the Rust/shim boundary is a real risk to probe.

## Decisions

## Dead ends

## Next step

Read the SDK headers (eikstart.h, aknapp.h, akndoc.h, aknappui.h, coecntrl.h) and nm the DSOs.

(old) Read CLAUDE.md, the Rust SDK design spec (§3, §5, §7, §8, §9, §11), experiments 65a/65, and `examples/gui/`.
