//! The proxy variables: production honours them, the tests' fetcher does not. They are
//! set only in a child test process, never in this one, where other tests run in parallel.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use super::server::{response, serve};
use crate::http_fetch::HttpFetch;
use crate::{Auth, Fetch, SourceSpec};

/// Set in the child; the child's proxy variables name the parent's proxy.
const CHILD: &str = "SYMDEV_TEST_PROXY_CHILD";
const NAME: &str = "http_fetch::tests::proxy::only_production_goes_through_the_proxy_variables";

#[test]
fn only_production_goes_through_the_proxy_variables() {
    if std::env::var_os(CHILD).is_some() {
        return in_the_child();
    }
    let (proxy, connects) = proxy();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy");
    for variable in [
        "ALL_PROXY",
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "all_proxy",
        "http_proxy",
        "https_proxy",
    ] {
        child.env(variable, &proxy);
    }
    let out = child.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "the child ran no test: {stdout}"
    );
    let connect = connects.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(connect.starts_with("CONNECT 127.0.0.1:"), "{connect}");
}

/// Every proxy variable names the parent's proxy: `new` must go through it, `direct`
/// must reach the server itself.
fn in_the_child() {
    let (server, _requests) = serve(vec![response("200 OK", b"from the server")]);
    let url = format!("{server}index.toml");
    let source = SourceSpec::new("public", &server, Auth::None).unwrap();
    let proxied = HttpFetch::new(&source, None).text(&url).unwrap();
    assert_eq!(proxied, "via the proxy");
    let direct = HttpFetch::direct("public", None).text(&url).unwrap();
    assert_eq!(direct, "from the server");
}

/// An HTTP proxy for one connection: it accepts the `CONNECT` that ureq sends and then
/// answers the tunnelled request itself. Returns its URL and the `CONNECT` line.
fn proxy() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, connects) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let head = |reader: &mut BufReader<_>| {
            let mut lines = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim_end().is_empty() {
                    return lines;
                }
                lines.push(line.trim_end().to_string());
            }
        };
        let connect = head(&mut reader);
        stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .unwrap();
        head(&mut reader);
        stream
            .write_all(&response("200 OK", b"via the proxy"))
            .unwrap();
        sender.send(connect[0].clone()).unwrap();
    });
    (url, connects)
}
