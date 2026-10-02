//! What bounds a request: the index's size caps a download, and a server that never
//! answers is given up on.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::server::{response, serve};
use crate::http_fetch::HttpFetch;
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
