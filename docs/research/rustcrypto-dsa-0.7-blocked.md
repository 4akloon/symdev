# Why the RustCrypto majors cannot be taken yet (2026-09-20)

Dependabot offers `der` 0.8, `x509-cert` 0.3, `pkcs8` 0.11, `sha1` 0.11 and `dsa` 0.7.
They are one generation and have to move together. The whole generation is blocked by
`dsa` 0.7, which no longer accepts the key size Symbian signing uses.

## What the keys are

`TraditionalDsaKey` from the experiment 5 material decodes to `p` = 128 bytes,
`q` = **28 bytes**, `g` = 128, `y` = 128, `x` = 28. That is DSA **L=1024, N=224** —
not one of the four NIST sizes.

## What `dsa` 0.7 accepts

`Components::from_components` matches the components against a fixed table
(`dsa-0.7.0/src/components.rs:47`), where the bit precision is limb-aligned
(`key_size.rs:38`, `Limb::BITS` = 64 here):

| constant | l_aligned | n_aligned |
|---|---|---|
| `DSA_1024_160` | 1024 | 192 |
| `DSA_2048_224` | 2048 | 256 |
| `DSA_2048_256` | 2048 | 256 |
| `DSA_3072_256` | 3072 | 256 |

Our key lands on (1024, 256). No row matches, so every construction returns
`signature::Error`. In `dsa` 0.6 there was no such table.

## Why the hazmat escape hatch is not enough

`Components::from_components_unchecked` (feature `hazmat`) skips the table, and with it
the hand-built path works: the `invalid DSA parameters` failures disappear. But the
decoders inside the crate still go through the checked path, so both of these keep
failing on real material:

- `VerifyingKey::from_public_key_der` → certificate SPKI, used by `SisBlob37::verify_dsa_sha1`;
- `SigningKey::from_pkcs8_der` → `BEGIN PRIVATE KEY` PEM.

Taking the generation would therefore mean hand-rolling SPKI and PKCS#8 decoding for DSA
to route around the crate's own parsers. That is not a dependency bump; it is owning a
fork of the crate's decoding, for keys the SDK tools produce.

## Verified with

`sha1` 0.11 is pinned to the same decision: it carries `digest` 0.11, while `dsa` 0.6
takes `digest` 0.10, so `Sha1` cannot be handed to `DigestSigner` across the split.
`rand` 0.10 is likewise tied to `dsa` 0.7's `TryCryptoRng`.

`cbc` 0.2, `des` 0.9 and `md-5` 0.11 are independent of this and were taken.

## Revisit when

`dsa` grows a public constructor or decoder for non-NIST sizes (or restores the 0.6
behaviour). Until then `.github/dependabot.yml` ignores the majors of the locked crates.
