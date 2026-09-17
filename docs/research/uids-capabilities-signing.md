# UIDs, capabilities, and signing

Bootstrap note for M0. Facts are **Verified** unless labeled **Unknown**. Promoted from spec §4.5, plus the self-sign / `makekeys -expdays 3650` facts from spec §4.2. No new argv.

## Capabilities (Verified)

Twenty capabilities, three tiers. **User-grantable (self-signable), exactly six:** `LocalServices`, `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location` (Location added in FP2).

System (7, need DevCert): `PowerMgmt`, `ProtServ`, `ReadDeviceData`, `SurroundingsDD`, `SwEvent`, `TrustedUI`, `WriteDeviceData`.

Manufacturer (3): `AllFiles`, `DRM`, `TCB`.

§17 and all self-signed builds **hard-error** on any capability outside the six. There is no opt-in identity in this spec. Until a later spec adds one, “refuse privileged capabilities unless the user opts in” is implemented as **always refuse**.

## UID policy (Verified)

- Default / hello: test range **0xE0000000–0xEFFFFFFF** (never required registration).
- Self-signed range **0xA0000000–0xAFFFFFFF** is allowed if the user sets `uid3` explicitly.
- Protected range **< 0x80000000** is a hard error under `signing.mode = "self-signed"` (will not install).

## Platform UID (Verified)

`.pkg` must include platform dependency **0x102752AE**. Omitting it produces “App is incompatible with phone” (Verified).

## Signing (Verified)

Signing is **self-sign only** in this product phase. Symbian Signed is closed (Verified). `makekeys -expdays 3650` requires Symbian 9.2+ tools (Verified). Without `-expdays` a certificate lasts about one year.

Never commit certificates or private keys.

## Unknown

`makekeys -dname` fields: **recorded in experiment 8** ([experiment-backlog.md](experiment-backlog.md); runbook chapter 9). This SDK `makekeys` usage lists `CN`, `C`, `O`, `OU`, `EM` (not spec placeholders `OR`/`CO`) and requires at least two of those attributes. A two-attribute minimum was not separately trialed.
