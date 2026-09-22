# Security

## Reporting a vulnerability

Please report security issues privately through GitHub's
[private vulnerability reporting](https://github.com/4akloon/symdev/security/advisories/new)
rather than in a public issue.

## Key material in this repository

`crates/symdev-sis/src/testdata/` contains **throwaway** DSA private keys (plain, 3DES-encrypted
and PKCS #8), a self-signed certificate and packages signed with them. They exist only as
fixtures for the tests of the SIS signer and are not used to sign anything else. Do not use them
to sign software you distribute; generate your own with `symdev package` (self-signed) or
`makekeys`.
