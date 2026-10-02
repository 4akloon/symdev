//! Where a request may be sent on: an `https` source never sends plain HTTP (so no
//! redirect can downgrade it), and a signed request follows no redirect at all.

use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::keys;
use super::server::{response, serve};
use crate::http_fetch::HttpFetch;
use crate::{Auth, Fetch, SdkError, SigV4, SourceSpec};

/// A port that reports every connection made to it.
fn watched_port() -> (u16, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (seen, connections) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            drop(stream);
            if seen.send(()).is_err() {
                break;
            }
        }
    });
    (port, connections)
}

#[test]
fn an_https_source_never_sends_a_plain_http_request() {
    let (port, connections) = watched_port();
    let source = SourceSpec::new("public", "https://pub.example/", Auth::None).unwrap();
    let fetch = HttpFetch::direct_for(&source, None);
    // What a redirect from the bucket to `http://` would ask for.
    let url = format!("http://127.0.0.1:{port}/index.toml");
    match fetch.text(&url) {
        Err(SdkError::Fetch { url: u, detail }) => {
            assert_eq!(u, url);
            assert!(detail.contains("https only"), "{detail}");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    let connected = connections.recv_timeout(Duration::from_millis(300));
    assert!(
        connected.is_err(),
        "the plain-http request reached the server"
    );
}

#[test]
fn a_signed_request_follows_no_redirect() {
    let (port, connections) = watched_port();
    let elsewhere = format!("http://127.0.0.1:{port}/elsewhere");
    let redirect = format!(
        "HTTP/1.1 302 Found\r\nLocation: {elsewhere}\r\nContent-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    let (base, _requests) = serve(vec![redirect.into_bytes()]);
    let source = SourceSpec::new("private", &base, Auth::S3).unwrap();
    let fetch = HttpFetch::direct_for(&source, Some(SigV4::s3(keys(), "auto")));
    let url = format!("{base}index.toml");
    match fetch.text(&url) {
        Err(SdkError::Fetch { detail, .. }) => assert_eq!(detail, "HTTP 302"),
        other => panic!("expected HTTP 302, got {other:?}"),
    }
    let connected = connections.recv_timeout(Duration::from_millis(300));
    assert!(
        connected.is_err(),
        "the signed request followed the redirect"
    );
}

#[test]
fn an_unsigned_http_source_still_follows_a_redirect() {
    let (target, _seen) = serve(vec![response("200 OK", b"schema = 1\n")]);
    let redirect = format!(
        "HTTP/1.1 302 Found\r\nLocation: {target}index.toml\r\nContent-Length: 0\r\n\
         Connection: close\r\n\r\n"
    );
    let (base, _requests) = serve(vec![redirect.into_bytes()]);
    let source = SourceSpec::new("mirror", &base, Auth::None).unwrap();
    let text = HttpFetch::direct_for(&source, None)
        .text(&format!("{base}index.toml"))
        .unwrap();
    assert_eq!(text, "schema = 1\n");
}
