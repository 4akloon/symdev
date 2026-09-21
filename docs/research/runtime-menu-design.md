# The Options menu at runtime: no manifest, no command id

Research note, 2026-09-21. Branch `runtime-menu`. Supersedes `command-id-design.md`,
which is deleted: the question that note answered — how a menu item's *number* reaches
the Rust source — no longer exists, because no number does. Cites experiment 91 and 95
in [experiment-backlog.md](experiment-backlog.md),
[avkon-rust-spec.md](avkon-rust-spec.md) §3.3/§6/§7,
`symbian-rs/crates/symbian-ui/src/menu.rs`, `symbian-rs/shims/s60/symrs_avkon.cpp`,
`crates/symdev-manifest/src/ui.rs`, `crates/symdev-build/src/ui_resources.rs`.

## The question, and the rule that settles it

Experiment 91 put the Options menu in `symdev.toml`:

```toml
[[ui.menu]]
id = "more"
label = "More bars"
```

and matched it in Rust with `Command::named("more")`, both sides hashing the same word
to the same number. `command-id-design.md` then spent a branch making the *word* safe:
`#[symbian_std::main(gui)]` read the manifest and emitted `menu::MORE`, so a misspelt
or deleted item was a compile error. All of that was solving a problem the design had
created.

The user's objection, verbatim: **«Навщо ці команди взагалі? Мені не подобається
використання symdev.toml, він має бути лише для конфігурації проєкту»**.

The rule that decides it: **the manifest holds what the phone needs before the
application runs.** uid3, capabilities, vendor, caption, icon — the registration
resource the launcher reads without launching anything — and the two compiled resources
the framework reads as the application starts, the button group and the menu bar. An
Options menu is only ever needed *while* the application runs. It does not belong
there.

## The API

```rust
fn menu(&self, m: &mut Menu<Self>) {
    m.item("More bars",  |app| app.bars += 1);
    m.item("Fewer bars", |app| app.bars -= 1);
    m.item("Reset",      |app| app.bars = 3);
    m.exit("Exit");
}
```

No ids, no constants, no `Command`. The label sits next to the code that acts on it.
The number is the line's position and never leaves `symbian-ui`.

`Menu<Self>` and not `Menu`: the action has to be typed by the application it mutates.
That is the one place the sketch in the brief had to grow a parameter.

## What the platform gives, all of it read from the headers

* `eikmenup.h:456` — `IMPORT_C void AddMenuItemL(const CEikMenuPaneItem::SData&)`,
  documented as adding a menu item **dynamically** and updating the scroll bar, with
  the warning that "`SData` is a structure so all fields in it should be set".
* `eikmenup.h:74-102` — `SData { TInt iCommandId; TInt iCascadeId; TInt iFlags;
  TBuf<40> iText; TBuf<1> iExtraText; }`, `ENominalTextLength = 40`.
* `eikmobs.h:43` — `IMPORT_C virtual void DynInitMenuPaneL(TInt aResourceId,
  CEikMenuPane* aMenuPane)` on `MEikMenuObserver`, which `CAknAppUi` implements.
* `eikon.hrh:221-239` — every `EEikMenuItem*` flag; `eikon.rh:103` — `STRUCT MENU_ITEM`
  defaults `command=0`, `cascade=0`, `flags=0`.

(`eikmenup.h` is non-ISO extended ASCII, so `ugrep` and the repo's default grep skip it
silently. `LC_ALL=C grep -a`.)

**A plain item is `iFlags = 0`, `iCascadeId = 0`, `iExtraText` empty** — not a guess:
those are the defaults the resource compiler itself gives `MENU_ITEM`, which is the
same structure by another route. `iExtraText` is where `CEikMenuPane` would show a
hotkey name.

## What was settled by observation

**`DynInitMenuPaneL` fires for an empty `MENU_PANE`.** `rcomp` compiles
`RESOURCE MENU_PANE r_symrs_menupane { items = { }; }`, the framework shows the pane,
calls the observer, and the four lines Rust adds appear (with Avkon's own "Show open
apps." above them, which the framework inserts itself).
`symbian-rs/corpus/95-runtime-menu/01-menu.png`.

**The resource cannot go away entirely.** Nothing calls `DynInitMenuPaneL` for a menu
bar that has no pane, and with no `menubar` at all in `EIK_APP_INFO` the left softkey
is an access violation — experiment 91's finding, re-verified on the current emulator
build by forcing `UiResources::has_menu()` to `false`: pressing F1 gives
`Thread Bars terminated … KERN-EXEC … exit code: 3`. So what stays compiled is a
`MENU_BAR` with one `MENU_TITLE` and an empty `MENU_PANE`, and nothing else.

**A label longer than 40 is cut in Rust, on a character boundary.** `iText` is a
`TBuf<40>` and `TDes16::Copy` of anything longer is a *descriptor panic*
(`ETDes16Overflow = 11`, `e32panic.h:131`), which no `TRAP` catches — the application
would simply die. `App::menu` has no error channel (it returns nothing, like
`Gc::text`), so the choice is truncation, and it is made before the descriptor is
reached: `Menu::add` encodes as many whole characters as fit into `[u16; MAX_LABEL]`.
The same `encode` compiled for the host gives 47 units → 40; a surrogate pair
straddling 40 → 39 units, never split; 50 units of surrogate pairs → 40. Proved on the
phone with a 47-character label: the pane rendered it ellipsised by Avkon, the
application did not panic, and selecting it still fired the right action
(`04-long-label.png`, `05-long-fired.png`). The shim clamps again at
`ENominalTextLength` because that is the descriptor's own invariant and it lives there;
with the Rust cut in place it never has anything to do.

## How the closure reaches the application

This is the part the previous slice could not do — the framework calls the app UI, not
the view, and the `ui-list` agent's callback could not reach the application struct.

The action is a **non-capturing closure**, which coerces to a `fn(&mut A)`: a `Copy`
value that borrows nothing. So `App::menu(&self, …)` can be called twice.

1. `DynInitMenuPaneL` → `vtbl::menu` → `app.menu(&mut Menu::fill(host, pane))`. Each
   `item` call adds a line whose command is `0x4000 + position`.
2. `HandleCommandL(n)` → `vtbl::command` → the position is `n - 0x4000` → the
   application declares its menu **again**, this time into `Menu::find(position)`,
   which touches nothing and only keeps the matching `fn` pointer.
3. The pointer is copied out, **the `&self` borrow ends**, and only then is it called
   with `&mut A`.

No `static mut`, no leak, no `Rc`, no boxed closure, and the two borrows are never live
at once. Declaring the menu twice is what `DynInitMenuPaneL` asks for anyway: the
framework calls it every time the menu opens, so a menu that depends on state is the
normal case, not an extra cost.

**Re-entrancy.** An action is handed the application and nothing else — no `Ui`, no
handle into C++ — so it cannot make a framework call, and the framework cannot produce
a nested callback into a frame that does not exist. That is why the repaint after an
action is the crate's (`vtbl::command` calls `Ui::redraw` once the action has returned)
rather than something the action asks for: handing it a `&Ui` would be handing it the
way back in. It is also why `m.exit` is a separate line rather than a closure that
calls `Ui::exit` — it carries `EEikCmdExit`, which the shim acts on itself, with no
Rust frame on the stack while the framework tears the application down.

## The softkeys, and the check experiment 91 needed

`softkeys = "options-exit"` stays in the manifest, and its meaning grows: it is now
also how an application says it *has* an Options menu. It has to be said there, because
the button group and the menu bar are compiled resources written before any Rust runs,
and nothing on the symdev side can see a Rust trait impl.

The manifest used to **refuse** `options-exit` without `[[ui.menu]]`. That refusal
guarded a real failure (re-verified above), and it is not deleted but made
unreachable: `UiResources` emits `menubar = r_symrs_menubar` and the `MENU_BAR`/
`MENU_PANE` pair from the *same* condition that puts `EAknSoftkeyOptions` on the left
button. One condition, so the two cannot disagree. The default is now `"exit"`, because
there is no menu in the manifest for it to follow.

What is still not checked, and cannot be from here: an application that writes
`softkeys = "options-exit"` and no `App::menu` gets a pane holding only Avkon's own
"Show open apps.". That is a pane, not a crash.

## What this deleted

* `crates/symdev-manifest/src/command_id.rs` (`CommandId`, the FNV hash and its vector
  table), `MenuItem`, `[[ui.menu]]`, the id-is-an-identifier rule and the
  constant-clash and hash-clash refusals.
* `symbian-rs/crates/symbian-ui/src/command.rs` — the whole `Command` type,
  `Command::named`, `Command::is`, `Command::EXIT`, `Command::OPTIONS` and the target
  side of the shared vector table. `App::command` with it.
* `symbian-rs/crates/symbian-macros/src/menu.rs` and the `symdev-manifest` path
  dependency it brought into every GUI application's host build (19 `Cargo.lock`
  entries; `symbian-rs/Cargo.lock` is back to 23 packages). `#[symbian_std::main(gui)]`
  reads no file again, and `Entry::shape` with it.
* `docs/research/command-id-design.md`.

Nothing of `Command` survives. The softkeys needed none of it: the right button's
`EEikCmdExit` and the left button's `EAknSoftkeyOptions` are both handled below the
crate, and `EAknSoftkeyEmpty` — what the left button sends when `softkeys = "exit"` —
is simply outside the position range and ignored.

## What it costs

`uidemo.exe` 12 844 → **13 714** (+870): the menu's labels and closures moved from a
resource into the image, which is the honest price of the label living next to its
code. Three GUI examples that declare **no** menu still grew: `notes` 15 298 →
**15 626**, `query` 20 120 → **20 449**, `ui-list` 14 489 → **14 773** (+284…+329).
That is `DynInitMenuPaneL` and `HostMenuItem` in `symrs_avkon.cpp`: the recorded GCCE
argv carries no `-ffunction-sections`, so a translation unit's whole `.text` is one
section and `--gc-sections` can only drop it whole (`libsymrs.a` 6 900 → 7 358 bytes of
`.text`). Splitting them into their own file would not help — the host table in
`symrs_avkon.cpp` names `HostMenuItem`, so the object is pulled either way. Every
console example is unchanged **to the byte**; `std-hello` moved 73 565 → 73 562, which
is the crate-disambiguator layout wobble experiment 87 recorded.

## What was not determined

* Whether `aResourceId` is worth checking in `DynInitMenuPaneL`. This application has
  exactly one pane, so any call is ours; a cascade menu or a second menu bar would need
  it, and neither has been built.
* Cascading submenus (`iCascadeId`) and dimmed/checkbox items (`iFlags`). The fields are
  set to the resource compiler's defaults and nothing else has been observed.
* Per-language labels. The old design's sketch (labels keyed by id in the manifest) is
  void with the manifest out of the picture; a Rust-side answer has not been designed.
