//! `HttpFetch` against a `TcpListener` on 127.0.0.1 that serves canned responses; nothing
//! here touches the network.

mod limits;
mod proxy;
mod redirects;
mod server;

use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

use self::server::{Request, response, serve};
use super::HttpFetch;
use crate::{AmzDate, Fetch, S3Keys, SdkError, SigV4};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn keys() -> S3Keys {
    S3Keys {
        access_key_id: "AKIDEXAMPLE".into(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// What `signer` produces for `method url` at the time the server saw in `x-amz-date`.
fn expected_signature(
    request: &Request,
    url: &str,
    extra: &[(&str, &str)],
    payload: &str,
    window: (u64, u64),
) -> Vec<(String, String)> {
    let seen = &request.headers["x-amz-date"];
    let now = (window.0..=window.1)
        .map(AmzDate::from_unix)
        .find(|date| date.as_str() == seen)
        .unwrap_or_else(|| panic!("x-amz-date {seen} is outside the call"));
    let signer = SigV4::s3(keys(), "auto");
    signer
        .sign(&request.method, url, extra, payload, &now)
        .unwrap()
}

#[test]
fn text_returns_the_body_of_a_get() {
    let (base, requests) = serve(vec![response("200 OK", b"schema = 1\n")]);
    let url = format!("{base}index.toml");
    let text = HttpFetch::direct("public", None).text(&url).unwrap();
    assert_eq!(text, "schema = 1\n");
    let request = requests.recv().unwrap();
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("GET", "/index.toml")
    );
    assert!(request.body.is_empty());
}

#[test]
fn download_writes_the_exact_bytes_over_an_older_file_and_counts_them() {
    let bytes: Vec<u8> = (0..=255u8).cycle().take(300_000).collect();
    let (base, _requests) = serve(vec![response("200 OK", &bytes)]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("a.tar.gz");
    std::fs::write(&dest, vec![7u8; 400_000]).unwrap();
    let url = format!("{base}gcce/12.1.0/a.tar.gz");
    let written = HttpFetch::direct("public", None)
        .download(&url, &dest, 300_000)
        .unwrap();
    assert_eq!(written, 300_000);
    assert_eq!(std::fs::read(&dest).unwrap(), bytes);
}

#[test]
fn a_body_cut_short_is_an_error_not_a_short_file() {
    let truncated =
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nonly ten b";
    let (base, _requests) = serve(vec![truncated.to_vec()]);
    let dir = tempfile::tempdir().unwrap();
    let url = format!("{base}a.tar.gz");
    let err = HttpFetch::direct("public", None)
        .download(&url, &dir.path().join("a.tar.gz"), 100)
        .unwrap_err();
    assert!(
        matches!(&err, SdkError::Fetch { url: u, .. } if *u == url),
        "{err}"
    );
}

#[test]
fn not_found_is_a_fetch_error_naming_the_url_and_the_status() {
    let (base, _requests) = serve(vec![response("404 Not Found", b"<Error/>")]);
    let url = format!("{base}index.toml");
    let err = HttpFetch::direct("public", None).text(&url).unwrap_err();
    match err {
        SdkError::Fetch { url: u, detail } => assert_eq!((u, detail.as_str()), (url, "HTTP 404")),
        other => panic!("{other}"),
    }
}

#[test]
fn forbidden_names_the_source() {
    let (base, _requests) = serve(vec![response("403 Forbidden", b"<Error/>")]);
    let url = format!("{base}index.toml");
    let err = HttpFetch::direct("private", None).text(&url).unwrap_err();
    assert!(
        matches!(&err, SdkError::Forbidden { url: u, source_name } if *u == url && source_name == "private"),
        "{err}"
    );
}

#[test]
fn an_unreachable_host_is_a_fetch_error_naming_the_url() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let url = format!("http://127.0.0.1:{port}/index.toml");
    let err = HttpFetch::direct("public", None).text(&url).unwrap_err();
    assert!(
        matches!(&err, SdkError::Fetch { url: u, .. } if *u == url),
        "{err}"
    );
}

#[test]
fn a_signed_get_carries_a_valid_r2_signature() {
    let (base, requests) = serve(vec![response("200 OK", b"schema = 1\n")]);
    let url = format!("{base}index.toml");
    let fetch = HttpFetch::direct("private", Some(SigV4::s3(keys(), "auto")));
    let before = unix_now();
    fetch.text(&url).unwrap();
    let after = unix_now();
    let request = requests.recv().unwrap();
    let authorization = &request.headers["authorization"];
    assert!(
        authorization.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"),
        "{authorization}"
    );
    assert!(
        authorization.contains("/auto/s3/aws4_request, "),
        "{authorization}"
    );
    assert_eq!(request.headers["x-amz-content-sha256"], EMPTY_SHA256);
    assert_eq!(request.headers["host"], base[7..base.len() - 1]);
    let expected = expected_signature(&request, &url, &[], EMPTY_SHA256, (before, after));
    for (name, value) in expected {
        assert_eq!(request.headers[&name], value, "{name}");
    }
}

#[test]
fn an_unsigned_get_carries_no_authorization() {
    let (base, requests) = serve(vec![response("200 OK", b"")]);
    HttpFetch::direct("public", None)
        .text(&format!("{base}index.toml"))
        .unwrap();
    let request = requests.recv().unwrap();
    assert!(!request.headers.contains_key("authorization"));
    assert!(
        !request
            .headers
            .keys()
            .any(|name| name.starts_with("x-amz-"))
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn put_file_sends_the_exact_body_with_signed_type_and_cache_headers() {
    let bytes: Vec<u8> = (0..=255u8).rev().cycle().take(200_000).collect();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("archive.tar.gz");
    std::fs::write(&file, &bytes).unwrap();
    let sha256 = sha256_hex(&bytes);
    let (base, requests) = serve(vec![response("200 OK", b"")]);
    let url = format!("{base}symdev-public/gcce/12.1.0/{sha256}.tar.gz");
    let fetch = HttpFetch::direct("public", Some(SigV4::s3(keys(), "auto")));
    let cache = "public, max-age=31536000, immutable";
    let before = unix_now();
    fetch
        .put_file(&url, &file, &sha256, "application/gzip", cache)
        .unwrap();
    let after = unix_now();
    let request = requests.recv().unwrap();
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.path,
        format!("/symdev-public/gcce/12.1.0/{sha256}.tar.gz")
    );
    assert!(request.body == bytes, "the server got other bytes");
    assert_eq!(request.headers["content-length"], "200000");
    assert_eq!(request.headers["content-type"], "application/gzip");
    assert_eq!(request.headers["cache-control"], cache);
    assert_eq!(request.headers["x-amz-content-sha256"], sha256);
    assert!(
        request.headers["authorization"].contains(
            ", SignedHeaders=cache-control;content-type;host;x-amz-content-sha256;x-amz-date, "
        ),
        "{}",
        request.headers["authorization"]
    );
    let extra = [
        ("content-type", "application/gzip"),
        ("cache-control", cache),
    ];
    for (name, value) in expected_signature(&request, &url, &extra, &sha256, (before, after)) {
        assert_eq!(request.headers[&name], value, "{name}");
    }
}

#[test]
fn put_file_refused_by_the_bucket_is_forbidden() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("index.toml");
    std::fs::write(&file, "schema = 1\n").unwrap();
    let (base, _requests) = serve(vec![response("403 Forbidden", b"<Error/>")]);
    let url = format!("{base}index.toml");
    let err = HttpFetch::direct("public", Some(SigV4::s3(keys(), "auto")))
        .put_file(
            &url,
            &file,
            &sha256_hex(b"schema = 1\n"),
            "application/toml",
            "no-cache",
        )
        .unwrap_err();
    assert!(
        matches!(&err, SdkError::Forbidden { source_name, .. } if source_name == "public"),
        "{err}"
    );
}

#[test]
fn put_file_of_a_missing_file_names_it_and_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.tar.gz");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let err = HttpFetch::direct("public", None)
        .put_file(
            &format!("http://127.0.0.1:{port}/missing.tar.gz"),
            &missing,
            EMPTY_SHA256,
            "application/gzip",
            "no-cache",
        )
        .unwrap_err();
    assert!(
        matches!(&err, SdkError::Io { path, .. } if *path == missing.display().to_string()),
        "{err}"
    );
}
