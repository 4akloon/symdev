# symdev-makekeys

Generates the self-signed DSA certificate and private key that `symdev package` signs a SIS with.
Native replacement for the SDK's `makekeys -cert`.

## Library

```rust,ignore
use std::time::SystemTime;
use symdev_makekeys::SelfSignedDsa;

let keys = SelfSignedDsa::generate(SystemTime::now())?;                 // the recorded example subject
let keys = SelfSignedDsa::generate_for("CN=Vendor,O=Vendor", SystemTime::now())?;
keys.cert_pem();   // X.509 certificate, DSA-with-SHA1, valid 3650 days
keys.key_pem();    // PKCS#8 "BEGIN PRIVATE KEY"
```

The issuer equals the subject. Keys are DSA 1024/160.

## Binary

```bash
makekeys -cert -expdays 3650 -password <pw> -len 2048 \
    -dname "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com" key.pem cert.cer
```

Only the recorded call is accepted: other `-expdays`, `-len` or `-dname` values, `-req` and
`-view` return `TODO: …`. The password must be at least 4 characters but is not used to encrypt
the output.

## Differences from the original

- The original writes an encrypted traditional `BEGIN DSA PRIVATE KEY` (DES-EDE3-CBC). This crate
  writes an unencrypted PKCS#8 key; `symdev-sis` can still *load* the original's encrypted keys
  when given the password.
- `makekeys -len 2048` produces a 2048/160 key. The `dsa` crate has no such size, so this crate
  generates 1024/160 regardless of `-len`.

`MakekeysTool` builds the argv for the original `makekeys.exe` under Wine, for comparisons only.

Never commit the generated `.key` or `.cer`.

## Testing

```bash
cargo test -p symdev-makekeys --offline
```
