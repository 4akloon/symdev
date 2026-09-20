# symdev-rcomp

Symbian resource compiler, natively: a C preprocessor for `.rss` sources, a lexer and parser for
the resource language, and a compiler that writes `.rsc` (compressed-Unicode text, Symbian's
packed integers) and the `.rsg` resource-ID header. Its output is byte-equal to the SDK's `cpp.exe`
plus `rcomp.exe` on the recorded corpus.

## Library

```rust,ignore
use symdev_rcomp::Rcomp;

let compiled = Rcomp::compile(preprocessed_rss_bytes, "app.rss")?;
std::fs::write("app.rsc", compiled.rsc_bytes()?)?;
std::fs::write("app.rsg", compiled.rsg_text())?;
```

- `CPreprocessor` runs `#include`, `#define`, conditionals and macros over the `.rss` first.
- `Rcomp` is the tool job (`-u`, `-o`, `-h`, `-s`, `-i`) and `Rcomp::compile` the core.
- `Rsc`, `RscAppRegistration`, `RscLtext16`, `RscUid` write the `.rsc` container and the
  application registration resource; `symdev-build` uses the latter when a project has no
  `_reg.rss`.

## Binary

```bash
rcomp -u -oapp.rsc -happ.rsg -sapp.rss.pp -iapp.rss
```

`-s` is the already-preprocessed source. `-u` is required: 8-bit resources are not implemented.
`-v`, `-p`, `-l`, `-force` and `-uid2/-uid3` return `TODO: …`.

## Not supported

Resource-language features and character sets that were never observed from the original compiler
fail with `TODO: … (not observed)` instead of producing a guess, for example some string escapes,
`CHARACTER_SET` values other than the recorded ones, and unusual data runs.

The language rules are written down in
[docs/research/rcomp-spec.md](../../docs/research/rcomp-spec.md).

## Testing

```bash
cargo test -p symdev-rcomp --offline
```
