# WIP: toolchain manager, Track B (sdk-net): SigV4, HttpFetch

Branch `tm-net`, worktree `~/worktrees/symdev/tm-net`. Plan:
`docs/superpowers/plans/2026-10-02-toolchain-manager.md` Track B (B1–B3); spec §4 Network.

## Status

- [x] B1 `SigV4`, `AmzDate` — `src/sigv4.rs` (+ private `sigv4/{canonical_request,request_target}.rs`), `src/amz_date.rs`, tests in `src/sigv4/tests.rs` and `src/sigv4/request_target/tests.rs`; 22 crate tests green
- [x] B2 `HttpFetch` GET — `src/http_fetch.rs`, `src/http_fetch/tests.rs`; 30 crate tests green
- [ ] B3 `HttpFetch::put_file`

## Facts

- SigV4 vectors (2026-10-02):
  - The live S3 page `https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html`
    now 302-redirects to the API index (the S3 API TOC and PDF no longer carry it). The
    official page's last archived copy is
    `https://web.archive.org/web/20251208134526id_/https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html`
    (gzip; title "Signature Calculations for the Authorization Header: Transferring Payload
    in a Single Chunk"). Four examples: GET Object (f0e8bdb8…), PUT Object (98ad7217…),
    GET Bucket Lifecycle (fea454ca…), Get Bucket List Objects (34b48302…), keys
    `AKIAIOSFODNN7EXAMPLE` / `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY`, 20130524T000000Z,
    us-east-1. The page prints `Authorization` without spaces after commas.
  - The generic SigV4 test suite zip (`docs.aws.amazon.com/general/latest/gr/samples/aws-sig-v4-test-suite.zip`)
    is 404; the IAM SigV4 pages (`IAM/latest/UserGuide/reference_sigv-*.html`) carry no
    vectors. AWS's own copy of `get-vanilla` lives in `awslabs/aws-c-auth`
    `tests/aws-signing-test-suite/v4/get-vanilla/` (context.json, header-canonical-request.txt,
    header-string-to-sign.txt, header-signature.txt, header-signed-request.txt):
    AKIDEXAMPLE, region us-east-1, service `service`, 20150830T123600Z, signs only
    host;x-amz-date, signature 5fa00fa3…, `Authorization` with `, ` separators.
- ureq 3.4.2 feature `rustls` = `rustls-no-provider` + `_ring` + `rustls-webpki-roots`:
  Mozilla roots are already compiled in (webpki-roots 1.0.9 in Cargo.lock), so no extra
  feature is needed. Proven 2026-10-02 with a throwaway example (deleted, not committed):
  `HttpFetch::new("probe", None).text("https://index.crates.io/config.json")` returned the
  JSON (`"dl": "https://static.crates.io/crates"`, `"api": "https://crates.io"`), `download`
  wrote 76 bytes, and a missing path gave `…/no-such-file-symdev: HTTP 404`. No proxy
  variables were set in that shell.
- ureq `Config::default()` reads the proxy from `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY`
  (+ lower-case) and `NO_PROXY`; `http_status_as_error` defaults to true; no timeouts by
  default. ureq-proto sets `Host` = URI host plus `:port` only when the port differs from the
  scheme default (`client/sendreq.rs` `maybe_with_port`); the signer must match that.
- ureq: an explicit `Content-Length` header makes the body sized (no chunked encoding),
  `SendBody`/`AsSendBody` is implemented for `File`; `Body::read_to_string` is lossy and
  capped at 10 MB, `as_reader` is unlimited.

## Decisions

- Canonical URI/query: percent-decode what the URL holds, then encode once with SigV4's
  unreserved set (S3 rule: no path normalisation, `/` kept), so `a;b`, `a%3Bb` both sign as
  `a%3Bb` (what S3 decodes on its side) and nothing is double-encoded.
- `sign` emits `Authorization` with `, ` separators (test-suite form); the S3 page's form
  without spaces carries the same signature, which the tests compare.

## Dead ends

- WebFetch of the S3 SigV4 page returns only the redirect stub; curl shows 302 to the API
  index. The S3 API PDF (`pdfs/AmazonS3/latest/API/s3-api.pdf`) has only SigV2 examples now.
- `smithy-rs` no longer keeps `aws-sig-v4-test-suite/get-vanilla/*.req` at the old path (404).

- `HttpFetch` maps 403 to `Forbidden` and every other non-2xx to `Fetch { detail: "HTTP <code>" }`
  with nothing appended, so E1's publisher can treat `HTTP 404` on `index.toml` as "no index
  yet" by matching the detail.
- Timeouts as the plan says: 30 s connect, 10 min per whole request (ureq `timeout_global`).
  Risk: a 58 MB GCCE archive needs ≥ ~100 KB/s to finish inside 10 min.
- Known limit: tests build `HttpFetch::new`, which takes the proxy from the environment; a
  shell with `HTTP(S)_PROXY`/`ALL_PROXY` set and 127.0.0.1 not in `NO_PROXY` would send the
  test requests through that proxy. No such variable is set here or in CI.

## Next step

B3: `HttpFetch::put_file` (tests first, same harness).
