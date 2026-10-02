//! An install over HTTP (spec §8): the source is a `Repo` served by a `TcpListener` in
//! the test, so nothing leaves 127.0.0.1.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use super::repo::Repo;
use super::{home, id};
use crate::{Auth, Host, SdkManager, SourceSpec, Sources};

/// Serves the files of `dir` for `requests` requests, one per connection; returns the
/// base URL and the paths asked for.
fn serve_dir(dir: PathBuf, requests: usize) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let (asked, paths) = mpsc::channel();
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split_whitespace().nth(1).unwrap().to_string();
            let mut header = String::new();
            while reader.read_line(&mut header).unwrap() > 2 {
                header.clear();
            }
            let file = dir.join(path.trim_start_matches('/'));
            let (status, body) = match fs::read(&file) {
                Ok(body) if !path.contains("..") => ("200 OK", body),
                _ => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
            asked.send(path).unwrap();
        }
    });
    (base, paths)
}

#[test]
fn installs_a_package_served_over_http() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let (base, paths) = serve_dir(tmp.path().join("repo"), 2);
    let sources = Sources {
        list: vec![SourceSpec::new("web", &base, Auth::None).unwrap()],
        file: "/config/symdev/sources.toml".into(),
    };
    let mut progress = Vec::new();
    let receipts = SdkManager::new(home(&tmp), sources, BTreeMap::new(), false, &mut progress)
        .unwrap()
        .direct_http()
        .ensure(&[id("gcce;12.1.0")])
        .unwrap();
    assert_eq!(receipts[0].source, "web");
    assert!(receipts[0].url.starts_with(&base), "{}", receipts[0].url);
    assert!(Repo::marker(&home(&tmp), "gcce;12.1.0").starts_with("gcce;12.1.0 from"));
    let asked: Vec<_> = paths.iter().take(2).collect();
    assert_eq!(asked[0], "/index.toml");
    assert!(
        asked[1].starts_with("/gcce/12.1.0/") && asked[1].ends_with(".tar.gz"),
        "{asked:?}"
    );
    let text = String::from_utf8(progress).unwrap();
    assert!(text.starts_with("installing gcce;12.1.0 ("), "{text}");
}
