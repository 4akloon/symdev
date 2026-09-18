# T2 native makekeys — replace Wine `makekeys` with self-signed DSA certs

Date: 2026-09-18
Branch: `t2-native-makekeys`
Worktree: `/home/genius/worktrees/symdev/t2-native-makekeys` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)

## Result

**hello.cer did not byte-match.** Frozen experiment-8 cert is DSA-SHA1 (not RSA), serial 1, Version 3 with no extensions, 2048-bit p / 160-bit q, subject the makekeys Example Usage DN, dates 2026-09-17 15:21:21 GMT → 2036-09-14 15:21:21 GMT. Regenerating cannot equal that file: new DSA parameters, RFC 6979 cert `k`, and `dsa` 0.6 has no 2048/160 `KeySize` (`DSA_2048_256` took ~192s per keygen in debug tests).

Native generate (injected timestamps; serial 1; recorded DN; OID `1.2.840.10040.4.3`) **self-verifies**. Native sign of the hello controller hash with the generated PKCS#8 key **verifies**. Subject is the recorded makekeys DN (`CN=Joe Bloggs` …), not the SIS pkg vendor string.

`SisPackage::package` writes `<name>.cer` / `<name>.key` natively when `signing.cert`/`signing.key` are absent (overwrite via `std::fs::write`, same as today). CLI `package` no longer calls `SisTools::from_env`. **Wine `makekeys` and `SYMDEV_EPOCROOT` are gone from package.**

No E52. Never `-fPIC`. No clap verbs. Default `cargo test` does not require Wine. PEM is not `SisEncode`.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `d04e7c5643e3aae51ac623c4406dcb7b7987a3be` |
| Native makekeys + experiment 40 | `1099339bde905ddb47908d730ac70dbee70574a3` |
| Frozen hello.cer PEM | SHA-256 `08818e08330d77a3e53dc7814082a9906ab7711c737d21aa09126f6b319c6f5e` |

Feature-branch tip SHA is the report commit on this branch.

## Wine makekeys argv (already in tree)

`SisTools::makekeys_args` / `makekeys_args_match_experiment_8` (experiment 8; password redacted):

```
/usr/bin/wine $EPOCROOT/epoc32/tools/makekeys.exe \
  -cert -expdays 3650 -password <pw> -len 2048 \
  -dname "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com" \
  hello.key hello.cer
```

## Key file format

Native sign (`encode_signed_sisx`) already reads PKCS#8 `BEGIN PRIVATE KEY`, unencrypted `BEGIN DSA PRIVATE KEY`, and Wine encrypted traditional DSA PEM (`DES-EDE3-CBC` + MD5 `EVP_BytesToKey`). Native makekeys **writes PKCS#8 PEM** (unencrypted). Wine `.key` encrypted traditional PEM is **not** re-emitted (OpenSSL DEK wrapping is undocumented makekeys output, not required for native sign). A Wine `.key` still loads when `signing.key` is set and `SYMDEV_SIGN_PASSWORD` is supplied.

## Wine / EPOCROOT

| Tool | Status |
|------|--------|
| `makesis` | not spawned (already native unsigned SIS) |
| `makekeys` | **not spawned** |
| `signsis` | not spawned (already native SISX) |

`SisTools::from_env` still requires `SYMDEV_EPOCROOT` for the recorded argv helpers. **CLI `symdev package` does not call it.** `makekeys_args` remains the experiment-8 recorded argv helper.

## Tests

TDD: `generate_self_signed_dsa` failed (missing function), then passed with injected hello Not Before. Package without Wine failed while `run_tool(makekeys)` still ran (`No such file or directory`), then passed and overwrote stale workdir `.cer`/`.key`. CLI `package` without `SYMDEV_EPOCROOT` failed on `missing toolchain: SYMDEV_EPOCROOT`, then succeeded with `SYMDEV_SIGN_PASSWORD`.

`cargo test --workspace --offline`:

- `symdev-build` 115 passed (injected-date generate + hello controller sign, native package cert/key without Wine, existing pair still skips generate, `makekeys_args` still matches experiment 8). Experiment-5 `hello.key` live sign returns early unless `SYMDEV_SIGN_PASSWORD` is set.
- `symdev` unittests 3 passed
- `cli` 19 passed (`package_missing_epocroot_still_packages`; password still required)
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. No `.sis` / `.sisx` / `.cer` / `.key` committed.

## Docs

- Experiment 40 (append-only): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t2-native-makekeys-report.md`
