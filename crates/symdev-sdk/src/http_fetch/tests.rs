//! `HttpFetch` against a `TcpListener` on 127.0.0.1 that serves canned responses; nothing
//! here touches the network.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::HttpFetch;
use crate::{AmzDate, Fetch, S3Keys, SdkError, SigV4};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// One request as the server read it; header names lower-cased.
struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

/// Serves each response on its own connection, in order; returns the base URL
/// (`http://127.0.0.1:<port>/`) and the requests as they arrive.
fn serve(responses: Vec<Vec<u8>>) -> (String, mpsc::Receiver<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            stream.write_all(&response).unwrap();
            sender.send(request).unwrap();
        }
    });
    (base, requests)
}

fn read_request(stream: &mut TcpStream) -> Request {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut start = line.split_whitespace();
    let method = start.next().unwrap().to_string();
    let path = start.next().unwrap().to_string();
    let mut headers = BTreeMap::new();
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let Some((name, value)) = line.trim_end().split_once(':') else {
            break;
        };
        headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
    }
    let length = headers
        .get("content-length")
        .map_or(0, |v| v.parse().unwrap());
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    Request {
        method,
        path,
        headers,
        body,
    }
}

fn response(status: &str, body: &[u8]) -> Vec<u8> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    [head.as_bytes(), body].concat()
}

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
    let text = HttpFetch::new("public", None).text(&url).unwrap();
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
    let written = HttpFetch::new("public", None)
        .download(&url, &dest)
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
    let err = HttpFetch::new("public", None)
        .download(&url, &dir.path().join("a.tar.gz"))
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
    let err = HttpFetch::new("public", None).text(&url).unwrap_err();
    match err {
        SdkError::Fetch { url: u, detail } => assert_eq!((u, detail.as_str()), (url, "HTTP 404")),
        other => panic!("{other}"),
    }
}

#[test]
fn forbidden_names_the_source() {
    let (base, _requests) = serve(vec![response("403 Forbidden", b"<Error/>")]);
    let url = format!("{base}index.toml");
    let err = HttpFetch::new("private", None).text(&url).unwrap_err();
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
    let err = HttpFetch::new("public", None).text(&url).unwrap_err();
    assert!(
        matches!(&err, SdkError::Fetch { url: u, .. } if *u == url),
        "{err}"
    );
}

#[test]
fn a_signed_get_carries_a_valid_r2_signature() {
    let (base, requests) = serve(vec![response("200 OK", b"schema = 1\n")]);
    let url = format!("{base}index.toml");
    let fetch = HttpFetch::new("private", Some(SigV4::s3(keys(), "auto")));
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
    HttpFetch::new("public", None)
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
