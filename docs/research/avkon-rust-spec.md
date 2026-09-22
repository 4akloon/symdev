# Running an Avkon application whose logic is Rust (design for step 75)

Status: **implemented**, 2026-09-21 — see [experiment 86](experiment-backlog.md) and
`symbian-rs/corpus/86-ui/`. Extended the same day by
[experiment 91](experiment-backlog.md) (`symbian-rs/corpus/91-ui-menu/`), which added
the Options menu and answered the softkey question 86 left open; its corrections are
marked **[91]**. Then by
[experiment 95](experiment-backlog.md) (`symbian-rs/corpus/95-runtime-menu/`), which
took the menu **out of the manifest**: it is declared in Rust and added to an empty
pane from `DynInitMenuPaneL`, and there is no command id in an application at all.
Its corrections are marked **[95]** and supersede the **[91]** ones they touch. Written 2026-09-20 on branch `ui-spec` as a specification;
the design held, and the places where the implementation contradicted it are marked
**[86]** in the text below rather than quietly rewritten. It plus
[experiment 76](experiment-backlog.md#76-the-avkon-shim-abi-a-thumb-c-shim-forwarding-to-arm-rust-t5-rust-sdk)
is what step 75 of the [Rust SDK design](../superpowers/specs/2026-09-20-rust-sdk-design.md)
was built from. Every SDK fact below was read from
`~/sdk/S60_3rd_FP2/epoc32/include/` or `nm -D` on `~/sdk/S60_3rd_FP2/epoc32/release/armv5/lib/`,
or observed from a probe compiled with symdev's own GCCE argv and run in EKA2L1. Where
something was not observed it says so; nothing is recalled.

Prerequisites from the design spec's §11 that this step assumes: 70 (the C++ shim and the
`TRAP` rule) and 73 (the async executor on the framework's scheduler). Experiment 76 shows
that the mechanics work without either, but a usable API needs both.

**[86] 73 was not a prerequisite.** A view that draws and handles keys is called *by* the
scheduler CONE already runs and awaits nothing, so step 75 shipped with 73 still open.
What 73 buys an Avkon application is work *between* callbacks.

---

## 1. The entry path

### 1.1 What a GUI `E32Main` does

The working C++ example in `examples/gui/src/gui.cpp` is the whole of it:

```cpp
LOCAL_C CApaApplication* NewApplication() { return new CAppApplication; }
GLDEF_C TInt E32Main() { return EikStart::RunApplication(NewApplication); }
```

From `epoc32/include/eikstart.h`:

```cpp
class EikStart
    {
public:
    IMPORT_C static TInt RunApplication(TApaApplicationFactory aApplicationFactory);
    };
```

`TApaApplicationFactory` (`apparc.h`) is a 16-byte value class — `TType iType; TUint iData;
mutable CApaApplication* iApplication; TInt iSpare2;` — with no user-declared copy
constructor or destructor, and an `IMPORT_C` converting constructor from
`typedef CApaApplication* (*TFunction)()`. So `RunApplication(NewApplication)` is two
imported calls: the constructor, then `RunApplication` by value.

The `E32Main` symbol itself is C++-mangled `_Z7E32Mainv` (the Rust SDK already exports it
by that name; `symbian-runtime::entry!`). For a GUI app the shim owns `E32Main` and the
Rust side never sees it — see §4.3.

### 1.2 Which symbol comes from which library

Observed, not assumed: `examples/gui` was built with the worktree's `symdev build` and its
`gui.elf` imports were counted per `NEEDED` DSO.

| Library in `gui.mmp`'s `LIBRARY` line | Imports in `gui.elf` | What the minimal app takes from it |
|---|---|---|
| `cone.lib` | 67 | every `CCoeControl` virtual and non-virtual the subclass's vtable names, `CCoeAppUi::AddToStackL`/`RemoveFromStack`/`HandleKeyEventL`, `CCoeEnv::Static` |
| `eikcore.lib` | 58 | `EikStart::RunApplication`, `CEikApplication`/`CEikDocument`/`CEikAppUi` bases, `CEikonEnv::TitleFont` |
| `avkon.lib` | 38 | `CAknApplication`, `CAknDocument(CEikApplication&)`, `CAknAppUi::BaseConstructL`, the `CAknAppUiBase` virtuals |
| `apparc.lib` | 6 | `TApaApplicationFactory(TFunction)`, `CApaApplication`/`CApaDocument` reserved virtuals |
| `euser.lib` | 11 | `User::AllocZ`/`AllocZL` (the `new (ELeave)` operators), `CBase::Extension_`, `TPtrC16(const TUint16*)`, `TRect::Height` |
| `gdi.lib` | 1 | `CFont::AscentInPixels` — the only `gdi` import in the whole example |
| (none) | — | `drtaeabi` 19 (`__cxa_*`, `_Unwind_VRS_*`, `__gxx_personality_v0`), `scppnwdl` 1 (`operator delete`) |

Two consequences worth naming, because they change the library list a Rust GUI app needs:

- **Drawing costs no library.** Every `CWindowGc` call the example makes (`Clear`,
  `SetPenColor`, `UseFont`, `DrawText`, `DiscardFont`) is a pure virtual of
  `CGraphicsContext` (`gdi.h` lines 2024–2914), so it is a vtable dispatch through the
  `CWindowGc&` the framework hands `Draw`. `ws32.lib` appears neither in the `LIBRARY`
  line nor in `NEEDED`. A shim that only draws through the gc it is given needs no window
  server import at all.
- **`gdi.lib` is needed only for the font object.** Drop `CFont::AscentInPixels` and the
  `gdi` import count falls to zero.

The `LIBRARY` line a Rust GUI project needs is therefore the same six as `gui.mmp`:
`euser apparc cone eikcore avkon gdi` — and because a Rust project has no `.mmp`, symdev
must put that list on the link line itself (§7.3).

### 1.3 Who owns the active scheduler

CONE owns it. `coemain.h` declares `class CCoeScheduler : public CBaActiveScheduler`
(`basched.h`: `CBaActiveScheduler : public CActiveScheduler`), and `CCoeEnv` has a private
`void CreateActiveSchedulerL();` plus a flag `ESchedulerIsRunning = 0x0004`. `CCoeEnv`
itself is `public CActive` — it is the window-server event sink *on* that scheduler, not
the scheduler. `CCoeEnv::ExecuteD()` is the call that runs it.

**Rule for step 73's executor, and it is not negotiable:** a GUI application must never
create or install a `CActiveScheduler`. By the time any Rust code runs, CONE has already
installed `CCoeScheduler` and is inside `CActiveScheduler::Start()`. The Rust executor
must attach to the running scheduler (`CActiveScheduler::Add` on shim-owned `CActive`
objects) and must never call `Start`/`Stop`. A console Rust app may own its scheduler; a
GUI one may not. The shim therefore needs two runtime entry shapes, and the manifest is
what selects them (§7).

---

## 2. The minimal subclass set

Read from the headers. "Pure" means the base leaves it `=0`, so the subclass must define
it; "default" means the base has an `IMPORT_C` body the subclass may keep.

### 2.1 `CAknApplication` (`aknapp.h` → `eikapp.h` → `apparc.h`)

| Virtual | Where declared | Status | Contract |
|---|---|---|---|
| `TUid AppDllUid() const` | `CApaApplication`, pure | **must define** | returns the app's UID3. Non-leaving, const, called early and often. |
| `CApaDocument* CreateDocumentL()` | `CEikApplication`, **private** pure, no arguments | **must define** | returns a new document; ownership passes to the framework. May leave. (C++ allows overriding a private virtual; the example does exactly this.) |
| `PreDocConstructL`, `OpenIniFileLC`, `NewAppServerL`, `Capability`, `ResourceFileName`, `BitmapStoreName`, `GetDefaultDocumentFileName`, `AppFullName` | `CAknApplication`/`CEikApplication`/`CApaApplication` | default | leave alone. `CAknApplication::PreDocConstructL` is what makes a second launch switch to the running instance. |

### 2.2 `CAknDocument` (`akndoc.h` → `eikdoc.h`)

| Virtual | Status | Contract |
|---|---|---|
| `CEikAppUi* CreateAppUiL()` | **must define** (pure on `CEikDocument`) | first-phase construction only: `new (ELeave) CMyAppUi`. Must **not** call the app UI's `ConstructL` — the framework does. Ownership passes to the framework. May leave. |
| constructor `CAknDocument(CEikApplication&)` | protected `IMPORT_C`, no default constructor | **must forward** | the subclass needs `CMyDocument(CEikApplication& a) : CAknDocument(a) {}`. |
| `OpenFileL` (both overloads), `NewDocumentL`, `StoreL`/`RestoreL`, `SaveL`, `IsEmpty`, `HasChanged` | default | a non-file-based app keeps every one. |

### 2.3 `CAknAppUi` (`aknappui.h` → `eikappui.h` → `coeaui.h`)

No pure virtuals anywhere on the chain. Everything is opt-in.

| Virtual | Status | Contract |
|---|---|---|
| `void ConstructL()` | default (`CAknAppUiBase::ConstructL` just calls `BaseConstructL`) | override to call `BaseConstructL(aFlags)` and build the view. **May leave**; the framework calls it inside its own trap harness. |
| `void HandleCommandL(TInt aCommand)` | default (`CEikAppUi`, empty) | menu/softkey commands. `EEikCmdExit = 0x100` (`eikon.hrh:376`); in `avkon.hrh:330-339` `EAknSoftkeyOptions = 3000`, `EAknSoftkeyBack = 3001`, and **`EAknSoftkeyExit = 3009`** — the enum runs Options, Back, Mark, Unmark, Insert, Yes, No, Done, Close, Exit. **[91] The "3002" this table carried until 2026-09-21 was recalled, not read, and 3002 is `EAknSoftkeyMark`.** **May leave.** |
| `TKeyResponse HandleKeyEventL(const TKeyEvent&, TEventCode)` | default (`CCoeAppUi`) | only for keys no stacked control claimed. **May leave.** |
| `void Exit()` | `IMPORT_C` virtual on `CEikAppUi`, overridden by `CAknAppUiBase` | call it, do not override. Non-leaving. |
| `TRect ClientRect() const` | `CEikAppUi`, non-virtual `IMPORT_C` | the area below the status pane and above the softkeys. |
| `void BaseConstructL(TInt aAppUiFlags = EStandardApp)` | `CAknAppUi`, non-virtual | flags from `CAknAppUiBase`: `EAknEnableSkin = 0x00080000`, `EAknEnableMSK = 0x00200000`, `EAppOrientationPortrait/Landscape/Automatic`, plus `CEikAppUi`'s `ENoAppResourceFile = 0x01`, `ENonStandardResourceFile = 0x02`, `ENoScreenFurniture = 0x04`. |
| `HandleResourceChangeL`, `HandleForegroundEventL`, `HandleWsEventL`, `HandleSystemEventL`, `HandleStatusPaneSizeChange`, `HandleError`, `MopSupplyObject` | default | leave alone for a first application; each is a later addition to the vtable (§4.4). |

Destruction: the app UI owns the view, so the subclass destructor must
`RemoveFromStack(iView); delete iView;` before the Rust object is dropped.

### 2.4 `CCoeControl` (`coecntrl.h`)

No pure virtuals. The ones a drawing, key-handling view needs:

| Virtual | Visibility in the base | Status | Contract |
|---|---|---|---|
| `void Draw(const TRect& aRect) const` | **private** `IMPORT_C` virtual | override | `aRect` is the region that needs redrawing. Called by the framework **outside any trap harness** — it must not leave, and it is `const`. The gc comes from `SystemGc()`, not from the argument. |
| `TKeyResponse OfferKeyEventL(const TKeyEvent&, TEventCode)` | public | override | returns `EKeyWasConsumed = 1` or `EKeyWasNotConsumed = 0` (`coedef.h`). **May leave.** |
| `void SizeChanged()` | protected | override when the layout is computed | non-leaving. |
| `TInt CountComponentControls() const` / `CCoeControl* ComponentControl(TInt) const` | public | override as a pair | a leaf view returns 0 and is done. |
| `void FocusChanged(TDrawNow)`, `PositionChanged()`, `HandlePointerEventL(const TPointerEvent&)` | protected/public | optional | E52 has no touch screen; pointer events are dead weight on this device but the vtable slot exists. |
| `ConstructL` | not a framework virtual at all | the subclass's own | the usual body is `CreateWindowL(); SetRect(aRect); ActivateL();` — all three are non-virtual `IMPORT_C` on `CCoeControl`. |

### 2.5 What subclassing costs at link time

`gui.o`, the single object of the four-class example, has **182 undefined symbols**. The
subclass's own vtable names every inherited virtual, including every `Reserved_*` slot of
`CCoeControl`, `CEikAppUi`, `MEikMenuObserver`, `MCoeMessageObserver` and
`MCoeViewDeactivationObserver`. That number is a property of the framework, not of how much
the application does: experiment 76's shim, which forwards seven virtuals to Rust instead
of implementing two, has 185. **Subclassing cost is fixed; forwarding is nearly free.**

---

## 3. The forwarding design

### 3.1 The three shapes

| Shape | What it is | Shim files | Adding a virtual | Binary cost |
|---|---|---|---|---|
| **A. One `extern "C"` function per virtual** | `symrs_view_draw(void* app, void* gc, …)` declared in the shim, defined in Rust | one `.cpp` per class, and a matching `extern` block per class in Rust | new symbol on both sides; a stale Rust SDK fails to **link** | one relocation and one PLT stub per virtual |
| **B. A vtable of function pointers, filled by Rust** | `struct SymRsAppVtbl { … }` in `.rodata`, reached through one exported function | one `.cpp` for the whole class set | new field; both sides must agree on the layout — needs a version word | one `.rodata` table, one PLT stub total |
| **C. One dispatch function with an opcode** | `i32 symrs_dispatch(void* app, u32 op, void* a, void* b)` | one `.cpp`, one `match` in Rust | new opcode constant, no signature change | smallest, but every argument is erased to `void*` and every return to `TInt` |

### 3.2 Recommendation: **B**, a Rust-owned vtable behind one exported symbol

Reasons, in the terms the task asks for:

- **Number of shim files.** B is one `shims/s60/avkon_app.cpp` holding all four
  subclasses. The classes are not independent — the document constructs the app UI, which
  constructs the view — so splitting them across files buys nothing and costs a header of
  cross-declarations. A is the same C++ but multiplies the declaration surface by the
  number of virtuals; C is one file too, but moves the type information out of the
  compiler's reach.
- **Binary size.** Measured, not argued: the four-class shim of experiment 76 compiles to
  30 996 bytes of `.o` and its E32 is 107 028 bytes, of which the vtable table itself is
  seven words. Per-virtual symbols (A) would add one dynamic relocation and one PLT stub
  each — the `uiprobe.elf` PLT already costs 8 bytes per imported symbol. Seven of them is
  noise; seventy, when the full `CCoeControl` surface is forwarded, is not.
- **Adding a virtual later.** This is where A and B differ most and where A looks better
  at first: a missing `extern "C"` symbol is a link error, a missing vtable field is
  undefined behaviour. The fix is to make B's table self-describing: the first field is
  `u32 size` (`sizeof(SymRsAppVtbl)`), the shim refuses to run if it is smaller than the
  fields it is about to call, and every field added goes on the end. That gives B a
  loud failure at startup instead of a silent one, and keeps a Rust SDK newer than the
  shim working. C cannot fail loudly at all: a wrong opcode is a runtime no-op.
- **Type safety.** B keeps the real signatures on both sides, so `#[repr(C)]` structs and
  `extern "C" fn` types are checked by both compilers. C throws that away.

### 3.3 The exact C ABI

Two tables, both `#[repr(C)]` / plain C `struct`, both in `.rodata`, both 32-bit-word
aligned (ARMv5TE, `+strict-align`; nothing in them is wider than a pointer).

```c
/* Value types, one-for-one with the SDK's. TKeyEvent is already a POD of four words
   (w32std.h): TUint iCode; TInt iScanCode; TUint iModifiers; TInt iRepeats. */
typedef struct { TInt x, y, w, h; } SymRsRect;
typedef struct { TUint code; TInt scan_code; TUint modifiers; TInt repeats; } SymRsKeyEvent;

/* "Down": what Rust may ask of the framework. Every entry is non-leaving because the
   shim TRAPs anything that can leave before returning (§5). */
typedef struct {
    TUint32 size;                                  /* sizeof(SymRsHost) */
    void (*clear)(void* gc, SymRsRect r);          /* [86] the rect form; see 5.1 */
    void (*set_pen_color)(void* gc, TUint32 argb);
    void (*set_brush_color)(void* gc, TUint32 argb, TInt solid);  /* [86] style + colour */
    void (*draw_rect)(void* gc, SymRsRect r);
    void (*draw_line)(void* gc, TInt x1, TInt y1, TInt x2, TInt y2);
    void (*draw_text)(void* gc, const TUint16* text, TInt len, TInt x, TInt y);
    void (*redraw)(void* view);
    void (*exit)(void* app_ui);
} SymRsHost;

/* "Up": what the framework calls on the Rust application object. */
typedef struct {
    TUint32 size;                                  /* sizeof(SymRsAppVtbl) */
    void* (*create)(void);
    void  (*destroy)(void* app);
    TInt  (*construct)(void* app, const SymRsHost* host, void* view, void* app_ui);
    void  (*draw)(void* app, void* gc, SymRsRect r);
    TInt  (*offer_key)(void* app, const SymRsKeyEvent* ev, TInt type);
    TInt  (*command)(void* app, TInt command);
    void  (*size_changed)(void* app, SymRsRect r);
} SymRsAppVtbl;

/* The single symbol the shim imports from the Rust side. */
extern "C" const SymRsAppVtbl* symrs_app_vtbl(void);
```

**Identity and lifetime of the Rust application object.**

- The object is an opaque `void*`, produced by `create()` and destroyed by `destroy()`.
  Rust allocates it (`Box::into_raw` once `alloc` is there, or a `static` in a `no_std`
  build with no allocator, as experiment 76 did) and Rust frees it. The C++ side never
  calls `delete` on it and never dereferences it.
- The shim stores it in three places, all of which live exactly as long as the app UI:
  `CShimAppUi::iApp`, `CShimView::iApp` (a borrow, not an owner) and nothing else. The
  view is destroyed before `destroy()` is called.
- Ordering, fixed by the framework: `create()` inside `CShimAppUi::ConstructL`, then the
  window-owning view, then `construct()` with the host table and the two opaque handles,
  then any number of `draw`/`offer_key`/`command`/`size_changed`, then the app UI
  destructor removes the view from the stack, deletes it, and calls `destroy()`.
- `gc`, `view` and `app_ui` are borrowed pointers valid only for the call (`gc`) or for the
  app UI's lifetime (`view`, `app_ui`). Rust must not store `gc`. A `Gc<'a>` wrapper with a
  lifetime tied to the `draw` call is the obvious safe façade.
- `host` outlives the app: it is a `const` static in the shim's `.rodata`.
- The vtable is `&'static` on the Rust side; `symrs_app_vtbl` may be called more than once
  and must return the same pointer.

**Thread and re-entrancy rules.** Every call in both directions happens on the app's one
thread, under CONE's active scheduler. `draw` may be entered while the Rust object is
already borrowed by an outer `offer_key` if the Rust code calls `redraw()` synchronously —
so `redraw()` must be documented as "schedules a redraw" (`DrawDeferred`, what experiment
76 used), never `DrawNow`, and the Rust façade should hold the state in a `RefCell`-free
struct reached by `&mut` only at the top of each callback.

**Verified.** Experiment 76 built exactly this ABI and ran it: a Thumb C++ shim calling an
ARM Rust `staticlib` through `blx <sym>@plt` and Rust calling back through `blx r1` on the
host table, inside a real Avkon application in EKA2L1
(`/tmp/claude-1000/ui-spec-work/uiprobe-1.png`).

### 3.4 Thumb / ARM interworking

symdev's compile argv (`crates/symdev-build/src/driver/compile.rs`) passes `-mthumb` and
`-mthumb-interwork`, so **the shim is Thumb-1** (`readelf -A shim.o`: `Tag_THUMB_ISA_use:
Thumb-1`), while rustc on `arm-symbian-e32` emits **ARM**. Experiment 76 settles what the
open experiment 62 left hanging for this boundary:

- Thumb → ARM direct call: the linker turns `bl probe76_run` into `blx <probe76_run@plt>`,
  and the PLT stub is ARM (`ldr pc, [pc, #-4]`). No hand veneer, no flag change.
- ARM → Thumb indirect call: rustc emits `blx r1` for every call through a function
  pointer, which is interworking-correct on ARMv5TE.
- The target JSON already carries `"has-thumb-interworking": true` and
  `asm-args: ["-mthumb-interwork", …]`.

One new link warning, harmless but worth naming: `uses 4-byte wchar_t yet the output is to
use 2-byte wchar_t`. Rust's object declares `Tag_ABI_PCS_wchar_t = 4`, the SDK's is 2.
Nothing in this ABI is a `wchar_t` — UTF-16 crosses as `*const u16` — so it is only an
attribute mismatch. It should be either silenced deliberately or fixed in the target JSON;
it must not be left as unexplained noise on every link.

---

## 4. Leaves, both directions

### 4.1 What a leave is here

`epoc32/include/variant/symbian_os_v9.3.hrh` line 651 defines `__LEAVE_EQUALS_THROW__`, and
that variant header is what symdev passes as `-D__PRODUCT_INCLUDE__`. So `e32cmn.h` takes
the `#else` branch and `TRAP` is:

```cpp
#define TRAP(_r, _s)  { TInt& __rref = _r; __rref = 0;
    try { TTrapHandler* ____t = User::MarkCleanupStack(); _s;
          User::UnMarkCleanupStack(____t); }
    catch (XLeaveException& l) { __rref = l.GetReason(); }
    catch (...) { User::Invariant(); } }
```

A leave is a real C++ exception, unwound by `drtaeabi`'s `_Unwind_*` with
`__gxx_personality_v0` — all 19 of `gui.elf`'s `drtaeabi` imports are that machinery.

### 4.2 Why a Rust frame must never be on the stack

Rust builds `panic = "abort"` with `default-uwtable = false`, and rustc still emits
`.ARM.exidx`. Experiment 65 recorded the `CANTUNWIND` entries; experiment 76 recorded what
they do. In `rawprobe.elf` the index has 20 entries and **one** covers the entire Rust text
region:

```
0x80ac <_Z7E32Mainv>: @0x35960   Personality routine: __gxx_personality_v0
0x813c <…probe76::note>: 0x1 [cantunwind]      <- covers 0x813c … 0x331d4, all of Rust
0x331d4 <CallThrdProcEntry>: @0x35994
```

With `User::Leave(-6)` called from the shim while a Rust frame was on the stack, the
emulator log stops at the note the Rust code printed a moment earlier and **the process
dies with no diagnostic at all** — no panic, no `KERN-EXEC`, nothing after that line. The
same build with the leave TRAPped inside the shim printed `Rust: trapped leave returned -6`
and carried on. This is not a style rule; it is the difference between an error value and
an unexplained disappearance.

### 4.3 The rule, per virtual

Two directions, two mechanisms, and they are not symmetric.

**Framework → Rust.** The shim's subclass method is on the stack when the framework calls
it. Whether the shim must open a trap harness depends on whether the framework already
opened one *and* on whether the Rust call can reach anything that leaves. The safe,
uniform rule is: **the shim never lets a leave start while a Rust frame exists**, which it
achieves by never calling a leaving SDK function from inside a Rust callback (§4.4) — the
`TRAP` goes around the *leaving call*, inside the host function, not around the Rust call.

**Rust → framework.** Every `SymRsHost` entry that touches something that can leave is
`TRAP`ped inside the shim and returns a `TInt`. Rust turns a non-`KErrNone` into its own
`Err`. Nothing throws while Rust is on the stack.

**Rust error → leave.** A Rust callback that must fail returns a `TInt`. The shim converts
it *after* the Rust frame has returned, with `User::LeaveIfError(err)` on a stack that is
pure C++ again. This is the only correct direction: Rust can never itself leave.

| Virtual the shim defines | Framework already trapped it? | Rust signature | What the shim does |
|---|---|---|---|
| `CShimApplication::AppDllUid() const` | n/a, non-leaving | not forwarded — the UID is a constant symdev generates into the shim | — |
| `CShimApplication::CreateDocumentL()` | yes (app startup) | not forwarded | `new (ELeave)`; may leave freely, no Rust on the stack |
| `CShimDocument::CreateAppUiL()` | yes | not forwarded | `new (ELeave)`; ditto |
| `CShimAppUi::ConstructL()` | yes | `construct(app, host, view, app_ui) -> TInt` | shim does `BaseConstructL`, `create()`, the view and `AddToStackL` itself (all leaving, no Rust frame), then calls `construct`, then `User::LeaveIfError(err)` **after** it returns |
| `CShimAppUi::HandleCommandL(TInt)` | yes | `command(app, cmd) -> TInt` | handles `EEikCmdExit`/`EAknSoftkeyExit` itself, else calls Rust and `User::LeaveIfError` after the return. **[95]** Rust reads the number as a position in the menu the application declares, and ignores anything outside that range |
| **[95]** `CShimAppUi::DynInitMenuPaneL(TInt, CEikMenuPane*)` | yes (`MEikMenuObserver`, through the menu bar) | `menu(app, pane) -> TInt` | the compiled `MENU_PANE` is empty; every line is added here, each time the menu opens, through the host entry `menu_item`, which traps `CEikMenuPane::AddMenuItemL` and returns its error. `User::LeaveIfError` after the Rust frame has returned |
| `CShimView::OfferKeyEventL(…)` | yes | `offer_key(app, &ev, type) -> TInt` | copies `TKeyEvent` into a `SymRsKeyEvent` (POD, four words), calls Rust, maps 0/1 to `EKeyWasNotConsumed`/`EKeyWasConsumed`. A Rust error has nowhere to go here; the design gives `offer_key` no error channel on purpose |
| `CShimView::Draw(const TRect&) const` | **no** — the framework calls `Draw` outside a trap harness | `draw(app, gc, rect)`, returns nothing | nothing in `draw` may leave, on either side. Every host entry reachable from `draw` must be non-leaving *by construction*, not by `TRAP` (a `TRAP` inside `Draw` would swallow an error nobody can report) |
| `CShimView::SizeChanged()` | no (non-leaving virtual) | `size_changed(app, rect)` | same as `Draw` |
| `CShimAppUi::~CShimAppUi()` | n/a | `destroy(app)` | destructors must not leave; `destroy` must not fail |

**Stack discipline, stated once.** At every instant, the stack is either entirely C++ below
the nearest `TRAP`, or it contains Rust frames and no exception may be raised. The shim
enforces this by making every function pointer in `SymRsHost` a complete `TRAP` unit: enter
C++, open the harness if the call can leave, close it, return a `TInt`. A host entry that
forgets its `TRAP` is exactly the `rawprobe` case, and `rawprobe` disappears silently.

### 4.4 Panics the other way

`panic = "abort"`. A Rust panic in a callback must not return into C++ with the framework's
invariants half-broken. `symbian-runtime`'s panic handler calls `User::Panic(_L("RUST"), KErrGeneral)` since
experiment 100 (it was `User::Exit(-1)`), the category `std` already used, so the emulator
reports a category for a console and a GUI program alike; only the console case has been
observed. Either way the process ends inside the panic handler and no C++
frame is unwound.

---

## 5. Drawing and input

### 5.1 What `Draw` gets

`void Draw(const TRect& aRect) const` — a **private** virtual of `CCoeControl`. `aRect` is
the invalid region; the gc comes from `CCoeControl::SystemGc()`, which returns a
`CWindowGc&`. `CWindowGc` derives from `CBitmapContext` from `CGraphicsContext`, and every
drawing entry point is a pure virtual of `CGraphicsContext` (`gdi.h`), so calling them
needs no import (§1.2).

The first Rust drawing API, in the order the header declares the primitives:

| Rust | `CGraphicsContext` virtual | Note |
|---|---|---|
| `gc.clear()` | **[86]** `Clear(const TRect&)` over the view's area, **never** the no-argument `Clear()` | clears with the brush; set the brush first or the result depends on what the framework left behind. The no-argument form is what produced experiment 76's black band: it leaves the top ~40 px of a window-owning control unpainted, isolated in experiment 86 with a red probe stripe and fixed by using the rect form |
| `gc.set_pen(Rgb)` | `SetPenColor(const TRgb&)` | `TRgb` is a one-word value class (`gdi.h`); cross it as `u32` |
| `gc.set_brush(Rgb)` | `SetBrushStyle(ESolidBrush)` + `SetBrushColor(const TRgb&)` | one Rust call, two gc calls: a null brush makes `draw_rect` an outline |
| `gc.rect(r)` | `DrawRect(const TRect&)` | filled with the brush, outlined with the pen |
| `gc.line(a, b)` | `DrawLine(const TPoint&, const TPoint&)` | |
| `gc.ellipse(r)` | `DrawEllipse(const TRect&)` | |
| `gc.text(&str, at)` | `UseFont(const CFont*)`, `DrawText(const TDesC&, const TPoint&)`, `DiscardFont()` | the shim owns the font (`CEikonEnv::Static()->TitleFont()`), so `UseFont`/`DiscardFont` never reach Rust; the text crosses as `(*const u16, len)` and the shim wraps it in a `TPtrC16` |
| `gc.text_in(&str, r, baseline, align)` | `DrawText(const TDesC&, const TRect&, TInt, TTextAlign, TInt)` | `TTextAlign` is `CGraphicsContext::TTextAlign` (`gdi.h` line 1671) |

That set is what experiment 76 implemented and what the screenshot shows working, minus
the ellipse and the aligned-text overload.

Coordinates: **not isolated.** The probe drew once with `Rs(Rect())` and once with
`Rs(TRect(TPoint(0,0), Size()))` and produced identical pixels, so which of the two is the
general rule for a window-owning control was not determined. The implementation must pin it
with a control whose rect does not start at the window origin.

**[86] Settled for the application, not for the framework.** The shim hands Rust
`TRect(TPoint(0,0), Size())` and nothing else: drawing through a `CWindowGc` is
window-relative, so a view that always lays out from `(0,0)` has no second coordinate
system to get wrong. A bar drawn at `y = area.height - 40` lands exactly above the
softkeys, and the measured client area is **240x245**. Which of the two `Rs()` forms the
framework itself means is still not determined, and no longer matters to an application.

### 5.2 How a key arrives

`CCoeAppUi::HandleKeyEventL` offers the event down the control stack; a control added with
`CCoeAppUi::AddToStackL(control, ECoeStackPriorityDefault, ECoeStackFlagStandard)` gets
`OfferKeyEventL(const TKeyEvent&, TEventCode)` and claims the event by returning
`EKeyWasConsumed`.

- `TEventCode` (`w32std.h` line 266): `EEventNull = 0`, `EEventKey = 1`, `EEventKeyUp = 2`,
  `EEventKeyDown = 3`. A character-producing key arrives as `EEventKeyDown`, then
  `EEventKey` with `iCode` set, then `EEventKeyUp`. Handle `EEventKey` and ignore the rest
  unless the application is a game.
- `TKeyEvent` (`w32std.h` line 974) is four words: `TUint iCode; TInt iScanCode; TUint
  iModifiers; TInt iRepeats;`. `iCode` is 0 on up/down events. `iModifiers` is
  `TEventModifier` (`e32keys.h`), e.g. `EModifierKeyUp = 0x00020000`,
  `EModifierAutorepeatable = 0x1`.
- `TKeyResponse` (`coedef.h`): `EKeyWasNotConsumed = 0`, `EKeyWasConsumed = 1`.

Codes, probed by compiling `e32keys.h` and printing the enumerators (not recalled):

| Key on an E52 | `TKeyCode` (`iCode`) | `TStdScanCode` (`iScanCode`) |
|---|---|---|
| Left softkey | `EKeyDevice0` 0xf842 | `EStdKeyDevice0` 0xa4 |
| Right softkey | `EKeyDevice1` 0xf843 | `EStdKeyDevice1` 0xa5 |
| Selection (D-pad centre) | `EKeyDevice3` 0xf845 | `EStdKeyDevice3` 0xa7 |
| D-pad left | `EKeyLeftArrow` 0xf807 | `EStdKeyLeftArrow` 0x0e |
| D-pad right | `EKeyRightArrow` 0xf808 | `EStdKeyRightArrow` 0x0f |
| D-pad up | `EKeyUpArrow` 0xf809 | `EStdKeyUpArrow` 0x10 |
| D-pad down | `EKeyDownArrow` 0xf80a | `EStdKeyDownArrow` 0x11 |
| Green / call | `EKeyYes` 0xf862 | `EStdKeyYes` 0xc4 |
| Red / end | `EKeyNo` 0xf863 | `EStdKeyNo` 0xc5 |
| Menu | `EKeyMenu` 0xf836 | `EStdKeyMenu` 0x94 |
| Application keys | `EKeyApplication0` 0xf852 … | `EStdKeyApplication0` 0xb4 … |

(`ENonCharacterKeyBase = 0xf800`; every `EKey*` above is `0xf800 + n`.)

Softkeys in practice: a softkey press is turned into a *command* by the button group
container declared by `EIK_APP_INFO`'s `cba` in the `.rss`, and arrives at
`HandleCommandL`, not at `OfferKeyEventL`. A Rust app that wants raw softkey scan codes
has to ask for them, and that is a later concern.

**[91] And the rest of this paragraph was wrong in the one way that cost three
sessions.** `R_AVKON_SOFTKEYS_EXIT` does *not* yield `EAknSoftkeyExit` on this ROM: it
draws `Exit` and delivers **3001**, `EAknSoftkeyBack`. Measured, with EKA2L1's log
filter changed from `Emulated.Stdout:off` to `trace` so a guest `RDebug::Print` is
visible at all, and with the probe on `CShimAppUi::HandleWsEventL`:

```
ws type=3 code=0    scan=a5      EStdKeyDevice1 down — offered to the view, declined
ws type=1 code=f843 scan=a5      EKeyDevice1 — consumed by the CBA, the view never sees it
HandleCommandL 3001              EAknSoftkeyBack
```

Two lessons, in order of importance:

1. **A softkey reaching `OfferKeyEventL` would be the bug.** The CBA is on the control
   stack at `ECoeStackPriorityCba` = 60 and the view at `ECoeStackPriorityDefault` = 0
   (`coeaui.h:47-61`), so the CBA is offered the `EEventKey` first and consumes it. The
   view sees only the `EEventKeyDown`/`Up` pair.
2. **Do not name a ROM CBA resource.** Whatever `R_AVKON_SOFTKEYS_EXIT` contains in this
   firmware — not determined; `avkon.rsc` is dictionary-compressed and was not decoded —
   the command ids in it are not symdev's to predict. A generated application declares
   its **own** `CBA` in its own `.rss` (§6.3), with `EEikCmdExit` on the right button and
   `EAknSoftkeyOptions` on the left. Both are compile-time constants out of the SDK's
   headers, so no resource numbering is involved.

`EAknSoftkeyOptions` is the one id that must not be replaced by a symdev-chosen number:
the framework itself watches for it and opens the menu bar instead of passing it on.
With `cba` naming an Options button and `menubar` naming nothing, F1 is
`Access violation reading address 0x9C`, with no `HandleCommandL` first.

**[86] Solved, by experiment 83, and this whole paragraph is now history.** Keys reach
the guest with `XSendEvent` to the emulator's toplevel window
(`docs/research/acceptance/emukey.py`), and step 75's acceptance is a pair of PID-bound
screenshots either side of `emukey.py keys <pid> Up Up`. **[91] The softkey half is
answered too** — see the paragraph above; an acceptance test may press F1 and F2. What
follows was true of the session that wrote this file.

**Not observed.** No key of any kind could be delivered to the emulated device from this
session: XTest key events with the emulator window activated (`_NET_ACTIVE_WINDOW` sent,
`XGetInputFocus` confirming the window) and the pointer warped over the screen produced
nothing — not the arrows, and not the stock Exit softkey (F2, bound to `EStdKeyDevice1` in
`~/.local/share/EKA2L1/bindings/default.yml`). The shim's `User::InfoPrint` at the top of
`OfferKeyEventL` never fired. So the key path is proven only structurally. **Finding a way
to drive keys into EKA2L1 is a prerequisite for step 75's acceptance**, and it is the first
thing to solve — before any shim code is written. Candidates, in order of cheapness:
EKA2L1's Lua scripting interface, a `--key` style CLI addition to our own fork, or its
touch-binding overlay (`~/.local/share/EKA2L1/bindings/touch/`).

EKA2L1's default host bindings, for whoever solves it:

| Host key (Qt code) | Symbian scan code |
|---|---|
| F1 (16777264) / F2 (16777265) | 164 / 165 = `EStdKeyDevice0` / `EStdKeyDevice1` |
| Enter (16777220) | 167 = `EStdKeyDevice3` |
| Up / Down / Left / Right (16777235 / …237 / …234 / …236) | 16 / 17 / 14 / 15 |
| F3 / F4 (16777266 / 16777267) | 180 / 181 = `EStdKeyApplication0` / `1` |

---

## 6. Resources

### 6.1 What a GUI app needs beside the EXE

From `examples/gui`, which installs and runs:

| File | Installed to | Content |
|---|---|---|
| `<app>.rsc` | `\resource\apps\` | `RSS_SIGNATURE`, a `TBUF`, `EIK_APP_INFO { cba = R_AVKON_SOFTKEYS_EXIT; }`, `LOCALISABLE_APP_INFO` with `short_caption`, `caption`, `number_of_icons` and `icon_file` |
| `<app>_reg.rsc` | `\private\10003a3f\import\apps\` | `UID2 KUidAppRegistrationResourceFile`, `UID3 <uid3>`, `APP_REGISTRATION_INFO { app_file; localisable_resource_file = "\resource\apps\<app>"; localisable_resource_id = R_APP_LOCALISABLE_APP_INFO; }` |
| `<app>_aif.mif` | `\resource\apps\` | the SVG icon, SVGB-encoded |
| `<app>.rsg` | build only | the `#define R_APP_LOCALISABLE_APP_INFO` the `_reg.rss` includes |

`AVKON_VIEW` and the view architecture are **not** needed for a single-view application and
should stay out of the first version: `examples/gui` has no view resource at all.

### 6.2 What symdev generates today

| For a C++ project (`bld.inf` + `.mmp`) | For a Rust project (`language = "rust"`) |
|---|---|
| `START RESOURCE` → `GcceBuild::compile_resource` (native `cpp` + `rcomp`, byte-equal on 143 SDK resources) | nothing |
| `[symbian] icon` → `GcceBuild::compile_icon` → `<app>_aif.mif` + `.mbg` | nothing |
| `SisPackage` adds every artifact with a `dest` | same code path, but `RustBuild::build` returns **only** `Artifact::exe(out)` |
| no project `_reg.rsc` → `symdev_rcomp::Rsc::registration(uid3, app)` — an `APP_REGISTRATION_INFO` with an **empty** `localisable_resource_file` and id 1 | same |

That generated registration is enough for a console app to be launchable by UID. It is
**not** enough for a GUI app: with no localisable resource the app has no caption and no
icon in the menu.

### 6.3 What the Rust path must start generating

`RustBuild` has to grow a resource stage. The cheap and evidence-backed way is to
**generate `.rss` text from the manifest and feed it to the existing native resource
compiler**, rather than to add new binary resource builders beside
`Rsc::registration`: `symdev-rcomp`'s `CPreprocessor::for_rss` + the compiler are already
byte-verified, and the text is a dozen lines.

Declaration in `symdev.toml` (new; everything else already exists):

```toml
[package]
name = "notes"

[language]
name = "rust"

[symbian]
uid3 = "0xe7351c7a"
icon = "gfx/notes.svg"        # already parsed; today only the C++ path uses it

[ui]                          # new section, present = this is a GUI application
kind = "avkon"                # the only value; selects the shim and the E32Main shape
caption = "Notes"             # LOCALISABLE_APP_INFO caption
short_caption = "Notes"       # optional, defaults to caption
softkeys = "exit"             # [95] or "options-exit", which is also how an
                              # application says it HAS an Options menu; default "exit"
left_softkey = "Options"      # [91] the label only; the command is fixed
right_softkey = "Exit"
```

**[95] There is no `[[ui.menu]]`, and no command id anywhere.** The rule that decides
what this section holds is *the manifest holds what the phone needs before the
application runs*: uid3, capabilities, vendor, caption, icon — the registration
resource the launcher reads without launching anything — and the two compiled
resources the framework reads as the application starts, the button group and the menu
bar. The Options menu itself is only ever needed **while** the application runs, so it
is declared in Rust (`App::menu`, §7.1) and its lines are added to an empty pane at
the moment it opens. A line is its label next to the code that acts on it; the number
`HandleCommandL` carries is the line's position, internal to `symbian-ui`.

That deletes the whole "two compilers must agree on a number" problem experiment 91's
`CommandId`/`Command::named` hash and the generated `menu` module existed to solve.
What is left of the range is where the positions start: `0x4000`, because below it is
everything the platform names (`EEikCmd*` at `0x100`, `EAknSoftkey*` at 3000–3200, the
reserved softkey ranges at `0x1000`/`0x1100`/`0x1200`), and `0x8000` is where
`CBA_BUTTON`'s **`WORD`** `id` (`eikon.rh:343`) would stop being representable.

and the stage produces, all under `build/`:

1. `<app>.rss` — the text of §6.1 with `caption`/`short_caption` substituted and
   `icon_file = "\\resource\\apps\\<app>_aif.mif"` when `[symbian] icon` is set
   (`number_of_icons = 0` and no `icon_file` when it is not), compiled with `HEADER` so
   `<app>.rsg` exists. **[91] It also carries the application's own `CBA` and, when
   `softkeys = "options-exit"`, its own `MENU_BAR` and an **[95]** empty `MENU_PANE`**
   (`eikon.rh:97-128` and
   `335-350`; there is no `AVKON_MENUBAR` struct, that name does not exist). The three
   named resources come *after* `EIK_APP_INFO`, which has to stay the third resource,
   and `EIK_APP_INFO` refers forward to them — `rcomp` resolves a forward `LLINK`, which
   is how every hand-written S60 `.rss` is laid out, and ours does too;
2. `<app>_reg.rss` → `<app>_reg.rsc`, with `localisable_resource_file` and
   `localisable_resource_id` filled in — this is the part `Rsc::registration` cannot do;
3. `<app>_aif.mif` + `.mbg` through `compile_icon`, which is already independent of
   anything GCCE-specific;
4. `Artifact`s with `dest` for all three, so `SisPackage` picks them up unchanged and its
   "no project `_reg.rsc`" fallback does not fire;
5. UID2 `0x100039CE` on the link and post-link (a GUI EXE's UID2), which the C++ path gets
   from the `.mmp`'s `UID` line and the Rust path has nowhere to get today — note that
   symdev's EXE `elf2e32` argv passes no `--uid2` at all and `examples/gui` runs anyway, so
   whether it matters on a device is **unverified**. **[86] Not done, deliberately**: the
   post-linker passes no `--uid2` for any EXE, C++ or Rust, and `examples/ui` installs,
   registers and runs without one. Inventing an argv flag to set a value nothing was
   observed to read would be a guess;
6. the six-library link list of §1.2, which `RustBuild::link_args` must add (it passes an
   empty `libraries` slice today). **[86] Done**, as `RustSdk::UI_LIBRARIES` through the
   recorded line's own `libraries` slot — five names, because `euser.dso` is already on
   it. The shim also needs the `epoc32/include` case-fold overlay, which `shims/common`
   does not: without it the Avkon chain stops at `fbs.h`'s `#include <FbsMessage.h>`.
7. **[86] not in this list and needed anyway:** `-u symrs_app_vtbl`. The shim archive
   follows the Rust archive, so the shim's one reference *back* into it would never
   resolve — ld 2.29.1 does not rescan.

The shim itself is a build input, not a generated file: one `.cpp` in the Rust SDK tree,
compiled with `GcceBuild::compile_args_for` (so it sees the same headers, the same
`gcce.h`, the same `GcceCompat` repair) and placed on the link line next to the Rust
archive. Experiment 76 did exactly that, by hand.

---

## 7. `examples/ui`: the smallest provable example

`symbian-rs/examples/ui`, built by `symdev build && symdev package && symdev run`.

**What it is.** One `CCoeControl`-backed view; **[91]** an Options menu of four items
and an application-owned `CBA`; **[95]** the menu declared in Rust, not in the manifest. Application state is a single `bars: u8` in the Rust
app struct.

**What it draws.** `clear`, then `bars` filled rectangles of increasing height along a
baseline, then the line, then the text `bars=<n> keys=<m>`. Exactly what experiment 76's
probe drew, because that picture is already known to render.

**What key it reacts to.** `EKeyUpArrow` (0xf809) increments `bars` up to 6,
`EKeyDownArrow` (0xf80a) decrements it down to 1; both return `EKeyWasConsumed` and ask for
a deferred redraw. Everything else returns `EKeyWasNotConsumed` so the softkeys still work.

**[95] What its menu is.** Four lines in `App::menu`, each a label next to the closure
that acts on it — `m.item("More bars", |app| …)`, `"Fewer bars"`, `"Reset"` — and
`m.exit("Exit")`, which carries `EEikCmdExit` so the shim ends the application itself.
No id, no constant, no number. (**[91]** had `Command::named("more")` and four
`[[ui.menu]]` entries in `symdev.toml`; both are gone.)

**How a run is verified.**

1. `symdev run` → `build/eka2l1.pid`, `build/eka2l1.log`.
2. `grep 'Found app: ui' build/eka2l1.log` — the registration resource worked.
3. A PID-bound screenshot per the `eka2l1-host` skill (resolve the window by
   `xwininfo -root -tree` + `xprop _NET_WM_PID`, capture with `XGetImage`) showing three
   bars and `bars=3 keys=0`.
4. Send `EKeyUpArrow` twice by whatever key path §5.2's open item settles on, screenshot
   again: five bars and `keys=2`. **Without that key path there is no pass**; a screenshot
   of the initial drawing alone repeats what experiment 76 already showed.
5. `grep -c 'Panic\|KERN-EXEC' build/eka2l1.log` is 0 and the emulator process exits on the
   Exit softkey rather than being killed.

A second, deliberately failing run is worth keeping in the record: a build in which one
host entry drops its `TRAP`, to confirm that the failure mode is still the silent death of
§4.2 and that nobody has quietly made it "work".

---

## 8. Cost and risks

### 8.1 Cost

Measured on experiment 76's probe, which is the real shape of the thing:

| Piece | Size |
|---|---|
| `shims/s60/avkon_app.cpp` — four subclasses, the host table, the ABI header | ~200 lines of C++, one file |
| `shims/s60/symrs_avkon.h` — the two structs and the one `extern "C"` declaration | ~40 lines, shared with the Rust side by hand (no bindgen; the structs are eight fields) |
| `symbian-ui` Rust crate — the vtable, the `Gc` façade, `Rect`/`KeyEvent`/`Rgb`, the `avkon_app!` macro | ~250 lines (**[86]** 7 files, ~600 lines with the documentation; there is no `avkon_app!` — `#[symbian_std::main(gui)]` writes the export and reads the application type from `fn main`'s return type) |
| `symdev-build` — the resource/icon stage for Rust, the library list, UID2 | ~200 lines of Rust plus tests |
| `.o` of the shim / E32 of the whole app | 30 996 B / 107 028 B |

**The 107 KB is the surprise and it is not the framework's fault.** `examples/hello` in
Rust is 752 bytes. The C++-only control of the same probe is 3591 bytes. Adding one C++
object next to the Rust archive pulls the whole `compiler_builtins` member — a single
compilation unit with libm and every `__aeabi_*` in it, not split per function — because
the C++ code references `mem*`. Design spec §4 hypothesis 4 predicted the collision
question; the answer is that there is no symbol collision, there is a 100 KB granularity
problem. Before step 75 ships, try `-Zbuild-std-features=compiler-builtins-mem` off,
`--gc-sections` on the link, or letting `euser.dso`'s strong `memcpy`/`memset`/`memmove`
win by placing the DSOs before the archive. Worth its own experiment; it is the difference
between a 5 KB and a 107 KB hello-world GUI app.

### 8.2 Risks, in order

1. ~~**Key injection into EKA2L1 is unsolved**~~ (§5.2). **[86] Solved by experiment 83**
   and used as the acceptance test; ~~the softkey half remains unexplained~~ **[91] the
   softkey half is answered**, and an application now owns its own `CBA`.
2. ~~**The `compiler_builtins` size cliff**~~ (§8.1). **[86] It did not come back**:
   `uidemo.exe` is 12 715 bytes (7 559 without the result-file harness) with a 31 KB C++
   object on the link line. Experiment 77's DSO ordering was the whole of the fix.
3. **Two runtime shapes.** A GUI app must not install an active scheduler and a console app
   must; `symbian-runtime`'s `entry!` currently assumes the console shape. The manifest's
   `[ui]` section is what has to select between them, and getting that wrong is a hang, not
   an error.
4. **Vtable versioning.** §3.2's `size` word is the whole defence. If it is skipped, a Rust
   SDK newer than the shim calls off the end of the table. **[86] Both tables carry it**
   and both sides check the other's — but **no short table was ever built**, so the
   check has never been seen to fire.
5. ~~**Drawing coordinates**~~ (§5.1) — **[86] settled for the application**: the shim
   passes an origin-zeroed rect and nothing else.
6. **`Draw` must not leave and must not panic.** It is the only forwarded virtual the
   framework calls outside a trap harness, and it is `const`. A Rust `draw` that allocates
   and OOMs has no legal way to report it.
7. **Nothing here has been on a device.** Hardware M0 is still open for C++; a GUI Rust app
   is further from the phone than anything symdev has built.

### 8.3 Experiments to run before implementation starts

| # | Question | Pass |
|---|---|---|
| A | How does a key reach the emulated device? | a script drives `EStdKeyUpArrow` into a running EKA2L1 and `examples/gui`'s softkey Exit closes the app |
| B | Can the E32 be brought back under ~10 KB with a C++ object present? | a hello-GUI E32 under 10 KB, with the mechanism recorded |
| C | Is `Draw`'s rect window-relative or screen-relative? | a control placed at a non-zero offset draws correctly with one of the two and visibly wrong with the other |
| D | Does the Rust executor of step 73 coexist with `CCoeScheduler`? | two `RTimer`s awaited from a GUI app's `construct`, with the view still repainting |

---

## 9. What could not be determined

- ~~**Key delivery** (§5.2)~~ **[86]/[91] settled.** Arrows and the selection key reach
  `OfferKeyEventL`; the softkeys reach `HandleCommandL` and never `OfferKeyEventL`,
  because the CBA is above the view on the control stack.
- **[91] What the ROM's `R_AVKON_SOFTKEYS_EXIT` actually contains.** It draws `Exit` and
  sends `EAknSoftkeyBack` (3001) rather than `EAknSoftkeyExit` (3009). Observed, not
  explained: the ROM's `avkon.rsc` is dictionary-compressed and was not decoded. It no
  longer matters to an application, which declares its own `CBA`.
- **[91] Non-ASCII in a generated resource.** `left_softkey`, a caption or a menu label
  may hold any text the manifest author wrote; only ASCII has been through `rcomp` and
  onto a screen. A Cyrillic label is **not observed** to survive the resource compiler.
- **Draw's coordinate origin** (§5.1): two different rects produced identical pixels.
- ~~**Why `gc->Clear()` left a black band** across the top of the client area in the
  probe.~~ **[86] Isolated, half-explained.** A red stripe at the top of the area the
  shim passes landed *at the top of the band*, so the band is inside the control; a
  filled `DrawRect(area)` in its place covered it. So the no-argument `Clear()` paints
  a smaller region than the control's area on this platform, and `Clear(const TRect&)`
  does not. *Why* its region is narrower is still not determined. It was not the brush.
- **Whether UID2 `0x100039CE` matters** (§6.3 item 5): symdev's EXE post-link argv passes
  no `--uid2` and `examples/gui` runs in the emulator regardless. Unverified on a device.
- **A transient install failure**: the first `symdev run` of the probe logged
  `Installation done!` and then `Installation of SIS failed`; an identical second run
  installed and ran. Not reproduced, not diagnosed. It is worth watching because it can
  make an unchanged binary look like a regression.
- **Everything device-side.** No symdev output has run on a stock E52, GUI or not.

---

## 10. Evidence index

| Claim | Evidence |
|---|---|
| Class hierarchy, pure vs default virtuals | `~/sdk/S60_3rd_FP2/epoc32/include/{aknapp,akndoc,aknappui,eikapp,eikdoc,eikappui,coeaui,coecntrl,apparc,eikstart}.h` |
| Which DSO exports what | `arm-none-symbianelf-nm -D` on `epoc32/release/armv5/lib/{apparc,cone,eikcore,avkon,gdi,euser}.dso` |
| Import census of a minimal Avkon app | `examples/gui/build/gui.o` (182 undefined) and `gui.elf` (`nm -u`, grouped by `@@<dll>`) |
| Key codes, `TKeyEvent`, `TEventCode`, `TKeyResponse` | probe over `e32keys.h`, plus `w32std.h` lines 266 and 974, `coedef.h` line 24 |
| `TRAP` is a C++ `try`/`catch (XLeaveException&)` | `e32cmn.h` line 5933, enabled by `__LEAVE_EQUALS_THROW__` in `variant/symbian_os_v9.3.hrh` line 651 |
| Thumb shim ↔ ARM Rust, both directions | experiment 76 stage A, `/tmp/claude-1000/ui-spec-work/probe76/` |
| A trapped leave returns a code; a raw one kills the process silently | experiment 76 stage A, both builds, EKA2L1 logs |
| Rust text is one `cantunwind` range | `readelf --unwind` on `rawprobe.elf` |
| A Rust `draw` callback paints a real Avkon view | experiment 76 stage B, `/tmp/claude-1000/ui-spec-work/uiprobe-1.png` |
| EKA2L1 host key bindings | `~/.local/share/EKA2L1/bindings/default.yml` |
| **[91]** A softkey reaches `HandleCommandL`, not `OfferKeyEventL`, and `R_AVKON_SOFTKEYS_EXIT` sends 3001 | experiment 91, `RDebug::Print` probes on `CShimAppUi::HandleWsEventL`/`HandleCommandL` with `Emulated.Stdout:trace`; `symbian-rs/corpus/91-ui-menu/README.md` |
| **[91]** `EAknSoftkeyExit = 3009`, `EAknSoftkeyBack = 3001` | `epoc32/include/avkon.hrh:330-339` |
| **[91]** An Options softkey with no menu bar is an access violation | experiment 91, EKA2L1 log `Access violation reading address 0x9C in thread Bars`; **[95]** re-verified on the current emulator build, `Thread Bars terminated … KERN-EXEC … exit code: 3` |
| **[95]** `DynInitMenuPaneL` fires for an **empty** compiled `MENU_PANE`, and `AddMenuItemL` fills it | experiment 95, `symbian-rs/corpus/95-runtime-menu/01-menu.png` |
| **[95]** `CEikMenuPaneItem::SData::iText` is a `TBuf<40>` and a longer `Copy` panics, not leaves | `eikmenup.h:76-95`, `e32panic.h:131` (`ETDes16Overflow = 11`) |
| What symdev generates today | `crates/symdev-build/src/driver/rust_build.rs`, `.../resource.rs`, `.../icon.rs`, `crates/symdev-build/src/package.rs`, `crates/symdev-rcomp/src/resource.rs` |
