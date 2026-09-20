# symdev-mbm

Symbian multi-bitmap stores: reads BMP files, converts them to a Symbian bitmap depth and writes
the `.mbm` file plus its `.mbg` header. Byte-equal to the SDK's `bmconv.exe` on the recorded
corpus. Library only; the build reaches it through `symdev-build` (`START BITMAP` blocks and
`.bmp` icon sources).

```rust,ignore
use symdev_mbm::{BmpImage, MbmBitmap, MbmDepth, MbmFile};

let image = BmpImage::parse(&std::fs::read("gfx/a.bmp")?)?;
let bitmap = MbmBitmap::compile(&image, MbmDepth::from_option("c24")?);
let mbm = MbmFile::new(vec![bitmap]).bytes()?;
```

- **`BmpImage`** parses 1, 4, 8, 24 and 32-bit bottom-up BMPs.
- **`MbmDepth`** covers `1`, `2`, `4`, `8`, `c4`, `c8`, `c12`, `c16`, `c24`, with the two
  built-in palettes.
- **`MbmRle`** compresses in the four schemes the format uses, and `MbmBitmap::compile` keeps
  the compressed form only if it is smaller.
- **`MbmFile`** writes the container and the `.mbg` header via `mbg_text`.

Top-down BMPs and other bit depths return `TODO: …`, since the original tool crashes or was not
observed on them. The format is described in
[docs/research/bmconv-spec.md](../../docs/research/bmconv-spec.md).

## Testing

```bash
cargo test -p symdev-mbm --offline
```
