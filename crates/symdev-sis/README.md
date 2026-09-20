# symdev-sis

Encodes Symbian SIS packages (the S60 3rd Edition format) and signs them, natively. It replaces
`makesis.exe` and, for the signing path `symdev package` uses, `signsis.exe`.

## What is in it

One type per SIS structure: `SisField`, `SisString`, `SisUid`, `SisVersion`, `SisDateTime`,
`SisLanguages`, `SisInfo`, `SisFiles`, `SisController`, `SisData`, `SisCompressed` (zlib),
`SisChecksum34` / `SisChecksum35`, `SisSignatures39`, and so on, each with the byte layout it was
observed to have.

The two entry points:

- **`SisUnsigned::encode(&SisUnsignedSpec)`** builds an unsigned SIS from an application name,
  UID3, version, vendor, the EXE, capabilities and a list of `SisPkgFile` (source bytes plus the
  install destination).
- **`SisUnsigned::encode_signed`** does the same and attaches a DSA-SHA1 signature made with a
  certificate and private key (PKCS#8, or the original tool's encrypted `DES-EDE3-CBC` PEM given
  the password). This is what `symdev package` calls, to produce a `.sisx`.

`SisTools` builds argv for the original `makesis.exe` / `signsis.exe` under Wine and is only used
to compare against them in tests.

## Binaries

```bash
makesis [-v] app.pkg [app.sis]
```

`makesis` reads a `.pkg` and writes an unsigned SIS. It understands only the shape symdev
itself writes: one `TYPE=SA` package, language `&EN`, platform UID `0x102752AE`, one EXE installed
to `!:\sys\bin\`, plus data files. `-h`, `-i`, `-s`, `-d` and any other package syntax return
`TODO: …`, and capability bits are not read from the E32.

```bash
signsis input output certificate key passphrase
```

`signsis` **does not work yet**: the binary parses its arguments and then stops (the library's
`Signsis::run` returns `TODO: inflate unsigned SIS and attach signatures`). Sign through
`SisUnsigned::encode_signed`, or `symdev package`.

## Testing

```bash
cargo test -p symdev-sis --offline
```
