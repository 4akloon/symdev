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

## Dead ends

## Next step

Write `symrs_query.cpp` and `query.rs`, plus `examples/query` as the driven evidence.
