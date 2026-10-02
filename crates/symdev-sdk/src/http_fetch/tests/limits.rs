//! What bounds a request: the index's size caps a download, and a server that never
//! answers is given up on.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::server::{response, serve};
use crate::http_fetch::HttpFetch;
use crate::http_timeouts::HttpTimeouts;
use crate::{Fetch, SdkError};

#[test]
fn a_download_longer_than_its_limit_stops_naming_the_url_and_the_sizes() {
    let (base, _requests) = serve(vec![response("200 OK", &[7u8; 300])]);
    let dir = tempfile::tempdir().unwrap();
    let url = format!("{base}a.tar.gz");
    let err = HttpFetch::direct("public", None)
        .download(&url, &dir.path().join("a.tar.gz"), 100)
        .unwrap_err();
    match err {
        SdkError::Fetch { url: u, detail } => {
            assert_eq!(u, url);
            assert!(detail.contains("more than the 100 bytes"), "{detail}");
        }
        other => panic!("{other}"),
    }
}

#[test]
fn a_server_that_never_answers_is_given_up_on() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/index.toml", listener.local_addr().unwrap());
    let (hang_up, hung_up) = mpsc::channel::<()>();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        let mut reader = BufReader::new(&stream);
        while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
            line.clear();
        }
        // Holds the connection, silent, until the test ends.
        let _ = hung_up.recv();
    });
    let (done, result) = mpsc::channel();
    thread::spawn(move || {
        let fetch = HttpFetch::impatient("public", None, Duration::from_millis(200));
        let _ = done.send(fetch.text(&url));
    });
    let outcome = result.recv_timeout(Duration::from_secs(10));
    drop(hang_up);
    match outcome {
        Ok(Err(SdkError::Fetch { detail, .. })) => assert!(detail.contains("imeout"), "{detail}"),
        Ok(other) => panic!("expected a timeout, got {other:?}"),
        Err(_) => panic!("still waiting for the response after 10 s"),
    }
}

/// Serves the headers of a `size`-byte body and its first ten bytes, then holds the
/// connection, silent, until the returned sender is dropped.
fn stall_after_ten_bytes(size: u64) -> (String, mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let (hang_up, hung_up) = mpsc::channel::<()>();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
            line.clear();
        }
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n");
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(&[7u8; 10]).unwrap();
        let _ = hung_up.recv();
    });
    (base, hang_up)
}

/// 0.2 s plus the size at 100 MB/s: a 10 MB body has 0.3 s, an index (10 MiB at most)
/// 0.305 s.
const QUICK: HttpTimeouts = HttpTimeouts {
    body_base: Duration::from_millis(200),
    body_rate: 100_000_000,
    ..HttpTimeouts::STANDARD
};

/// The server answered, so the 60 s for the answer is spent; the body is bounded by its
/// size at the minimum rate, not by the hour.
#[test]
fn a_download_whose_body_stalls_is_given_up_on_by_its_size() {
    let (base, hang_up) = stall_after_ten_bytes(10_000_000);
    let url = format!("{base}a.tar.gz");
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("a.tar.gz");
    let (done, result) = mpsc::channel();
    let started = Instant::now();
    thread::spawn(move || {
        let fetch = HttpFetch::with_timeouts("public", QUICK);
        let _ = done.send(fetch.download(&url, &dest, 10_000_000));
    });
    let outcome = result.recv_timeout(Duration::from_secs(10));
    let waited = started.elapsed();
    drop(hang_up);
    match outcome {
        Ok(Err(SdkError::Fetch { detail, .. })) => {
            assert!(
                detail.contains("download stopped after 10 bytes"),
                "{detail}"
            );
            assert!(detail.contains("imeout"), "{detail}");
        }
        Ok(other) => panic!("expected a timeout, got {other:?}"),
        Err(_) => panic!("still downloading after 10 s"),
    }
    // The base alone is 0.2 s: the size counted.
    assert!(
        waited >= Duration::from_millis(300),
        "gave up after {waited:?}"
    );
}

/// An index is bounded the same way, by the most it may be.
#[test]
fn an_index_whose_body_stalls_is_given_up_on() {
    let (base, hang_up) = stall_after_ten_bytes(100_000);
    let url = format!("{base}index.toml");
    let (done, result) = mpsc::channel();
    thread::spawn(move || {
        let fetch = HttpFetch::with_timeouts("public", QUICK);
        let _ = done.send(fetch.text(&url));
    });
    let outcome = result.recv_timeout(Duration::from_secs(10));
    drop(hang_up);
    match outcome {
        Ok(Err(SdkError::Fetch { detail, .. })) => assert!(detail.contains("imeout"), "{detail}"),
        Ok(other) => panic!("expected a timeout, got {other:?}"),
        Err(_) => panic!("still reading the index after 10 s"),
    }
}
