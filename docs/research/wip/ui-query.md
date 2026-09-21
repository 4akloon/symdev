# WIP: Avkon query dialogs (branch `ui-query`, backlog 93)

Task: add modal Avkon query dialogs (text / number / confirmation) to `symbian-ui` as
`symbian-rs/crates/symbian-ui/src/query.rs` + `symbian-rs/shims/s60/symrs_query.cpp`,
with a Rust surface that names no Symbian type, and drive one in EKA2L1 as evidence.

## Findings

- `aknquerydialog.h` (SDK `epoc32/include/`, CRLF/ISO-8859 — grep needs `-a`) declares
  `CAknQueryDialog` (base, `CAknDialog` + `MAknQueryControlObserver`) with overloaded
  static `NewL` for each value type: `NewL(const TTone&)`, `NewL(TDes&)`, `NewL(TInt&)`,
  `NewL(TTime&)`, `NewL(TTimeIntervalSeconds&)`, `NewL(TReal&)`, `NewL(TInetAddr&)`,
  `NewL(TPosition&)`. Concrete classes: `CAknTextQueryDialog` (520),
  `CAknNumberQueryDialog` (682), `CAknTimeQueryDialog` (787),
  `CAknDurationQueryDialog` (900), `CAknFloatingPointQueryDialog` (1005),
  `CAknMultiLineDataQueryDialog` (1124).
- `CAknQueryDialog::ExecuteLD(TInt aResourceId, const TDesC& aPrompt)` exists (line 228),
  so the prompt does not need a separate `SetPromptL`.
- Resource ids **already in the ROM's `avkon.rsg`**: `R_AVKON_DIALOG_QUERY_VALUE_TEXT`
  0x8cc0052, `..._NUMBER` 0x8cc0053, `..._PHONE`, `..._TIME`, `..._DATE`, `..._DURATION`.
- **No confirmation-query resource id exists in `avkon.rsg`.** `AVKON_CONFIRMATION_QUERY`
  is only a resource STRUCT in `avkon.rh` line 165, i.e. a shape an application
  instantiates in its own `.rss`. A confirmation query therefore needs a generated
  resource; text and number do not.
- `eikdialg.h` on `ExecuteLD`: "destroys the dialog when it exits, therefore there is no
  need for the application program to destroy the dialog". It says **nothing** about the
  leaving path.
- `shim_sources()` in `crates/symdev-build/src/rust_sdk.rs` globs `shims/s60/*.cpp`, so a
  new `symrs_query.cpp` is compiled with no build-system edit.
- `nm -D` on `epoc32/release/armv5/lib/avkon.dso` exports both `ExecuteLD` overloads:
  `_ZN15CAknQueryDialog9ExecuteLDEi` and `_ZN15CAknQueryDialog9ExecuteLDEiRK7TDesC16`,
  and the factories `_ZN15CAknQueryDialog4NewLER6TDes16RKNS_5TToneE` (text),
  `_ZN15CAknQueryDialog4NewLERiRKNS_5TToneE` (number) and
  `_ZN15CAknQueryDialog4NewLERKNS_5TToneE` (no value).
- `R_AVKON_SOFTKEYS_YES_NO` (0x8cc0024) and `R_AVKON_SOFTKEYS_OK_CANCEL` (0x8cc0014) are
  in the ROM's `avkon.rsg`; the confirmation *dialog* that would use them is not.
- `CAknGlobalConfirmationQuery` is exported by `aknnotify.dso`, not `avkon.dso`, is
  asynchronous and is a *global* (notifier-server) query, so it is not the in-app
  confirmation this surface means.

## Decisions

- Shape **A**, not the `SymRsHost` table: three free `extern "C"` hidden symbols from
  `symrs_query.cpp`, declared in `query.rs`. `symrs_avkon.h`/`abi.rs` are the menu
  agent's and stay untouched; a query needs no view, no app UI and no host entry.
- Text uses `CAknQueryDialog::NewL(TDes16&)` + `ExecuteLD(R_AVKON_DIALOG_QUERY_VALUE_TEXT,
  prompt)`, number `NewL(TInt&)` + `ExecuteLD(R_AVKON_DIALOG_QUERY_VALUE_NUMBER, prompt)`.
  The ids come from `#include <avkon.rsg>` in the shim, not typed out.
- The maximum input length is a **required argument** of `query::text(prompt, max_chars)`.
  The shim wraps the caller's buffer in a `TPtr16(buf, 0, max)`, so the descriptor the
  dialog writes into is the caller's and its max length is the bound. Nothing defaults.
- `ExecuteLD` under a `TRAP`: the safe reading. On a leave the shim does **not** delete
  the dialog — `eikdialg.h` documents destruction on exit and says nothing about the
  leaving path, and a double delete is worse than a leak.

- Built: `querydemo.exe` **18 889** bytes. `symrs_query.o` imports exactly the three
  cited mangled names. `uidemo` is still **12 715** and `hello-raw` still **752** — the
  new translation unit costs a program that asks no question nothing, because the shim
  is an archive and an unreferenced member is never pulled.

- **Driven in EKA2L1, pid-bound.** `Return` (the selection key, `EKeyDevice3`) opens the
  text query; the dialog draws prompt "Name?", an empty editor and RSK "Cancel" with no
  LSK. Three `5` presses put `555` in the field and the LSK turns into "OK". A second
  `Return` **confirms the dialog** — so a data query needs no softkey, and the confirm
  path is *not* blocked on the softkey slice.
- The answer reaches Rust: the view then draws `name=555`, i.e. the `TPtr16` the shim
  built over the Rust `Vec<u16>` is the descriptor `CAknTextQueryDialog` wrote into.

- A number query opens with the `initial` this side passed (`7`, selected in the field),
  takes `4` `2` and comes back as `age=42`.
- **The softkeys work on a query dialog.** `F2` on an open query cancels it — the Rust
  side sees `Ok(None)` and the cancel counter goes to `x1` — and `F1` confirms one
  (`name=99` after `9` `9` `F1`). So `eka2l1-input.md`'s "softkeys do nothing in our own
  applications" is narrower than it reads: it is about a CBA built from **our generated
  `.rss`**. A dialog whose CBA comes from the ROM's own resource takes both softkeys.
  Worth handing to the softkey slice as a discriminator.
- `Escape` does nothing because it is **not bound**: the profile at
  `~/.local/share/EKA2L1/bindings/default.yml` has 22 binds and they are F1-F4, Return,
  the four arrows, `0`-`9`, `*`, `/` and Backspace. Nothing else reaches the guest.

- **A query can run from `construct`.** A throwaway build that opened
  `query::text("FromConstruct?", 32)` from the `construct` callback showed the dialog
  during startup, took `7` `7` and `Return`, came back `Some("77")`, and the application
  carried on normally. So `CCoeEnv` already exists when `CShimAppUi::ConstructL` calls
  into Rust — CONE builds the environment before the app UI — and `construct` simply
  blocks inside the dialog's own loop until it is dismissed. The probe was reverted.

## Dead ends

## Next step

Run `examples/query` in EKA2L1 and drive a dialog with `emukey.py`.
