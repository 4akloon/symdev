# symdev-mif

Symbian icon files: converts SVG Tiny to the binary SVG the phone renders (`.svgb`, as
`svgtbinencode` writes it) and packs icons into a `.mif` container together with the `.mbg` header
that names them. Byte-equal to the SDK's `mifconv.exe` on the recorded corpus. Library only; the
build reaches it through `symdev-build`.

```rust,ignore
use symdev_mif::{MifFile, Svgb, SvgElement};

let svgb = Svgb::new(Svgb::MIFCONV_VERSION)?.encode(&svg_root)?;   // SvgElement → bytes
// wrap the bytes in a MifIcon at a MifDepth (e.g. "c32,8"), then:
let mif = MifFile::new(icons).bytes()?;
let mbg = MifFile::new(icons).mbg_text("app_icons.mbg");
```

## Modules

- **`svg`** — `SvgElement`, a small SVG Tiny document tree.
- **`svgb`** — `Svgb`, the encoder: fixed-point numbers, colours, paint, paths, transforms and the
  attribute tokens of the icon subset (version 3 is `mifconv`'s default).
- **`mif`** — `MifFile`, `MifIcon`, `MifIconData`, `MifDepth` (`1`, `2`, `4`, `8`, `c4`, `c8`,
  `c12`, `c16`, `c24`, `c32`, with an optional `,1` / `,8` mask).

## Not supported

Anything outside the icon subset that `svgtbinencode` was pinned down for is refused with
`TODO: … (not observed)`: unknown elements and attributes, colours missing from its keyword table,
`preserveAspectRatio`, most transforms, non-pixel units. Bitmap (`.bmp`) sources are handled by
`symdev-mbm`.

The format is described in [docs/research/svgb-mif-spec.md](../../docs/research/svgb-mif-spec.md).

## Testing

```bash
cargo test -p symdev-mif --offline
```
