use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::time::Duration;

use super::{ControlClient, ExitType};

/// Serves one connection: for each request line, `answer(line)` gives the lines to send.
fn fake(
    answer: impl Fn(&str) -> Vec<String> + Send + 'static,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("c.sock");
    let listener = UnixListener::bind(&sock).unwrap();
    std::thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        let mut w = s.try_clone().unwrap();
        for line in BufReader::new(s).lines() {
            for out in answer(&line.unwrap()) {
                writeln!(w, "{out}").unwrap();
            }
        }
    });
    (dir, sock)
}

fn id_of(line: &str) -> String {
    line.split("\"id\":")
        .nth(1)
        .unwrap()
        .split([',', '}'])
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn install_launch_and_an_exit_that_arrives_between_answers() {
    let (_d, sock) = fake(|l| {
        let id = id_of(l);
        if l.contains("\"package.install\"") {
            assert!(l.contains("\"path\":\"/abs/app.sisx\""), "{l}");
            vec![format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{}}}}"
            )]
        } else if l.contains("\"app.launch\"") {
            assert!(l.contains("\"uid\":\"0xE1234567\""), "{l}");
            vec!["{\"jsonrpc\":\"2.0\",\"method\":\"event.app_exited\",\"params\":{\"uid\":3792946535,\"pid\":7,\"name\":\"app\",\"exit_type\":\"panic\",\"exit_reason\":3,\"exit_category\":\"RUST\"}}".into(),
                 format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{\"pid\":7}}}}")]
        } else {
            vec![format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{{\"events\":[\"app_exited\"]}}}}"
            )]
        }
    });
    let mut c = ControlClient::connect(&sock).unwrap();
    c.subscribe_app_exited().unwrap();
    c.install(std::path::Path::new("/abs/app.sisx")).unwrap();
    assert_eq!(c.launch(0xe123_4567).unwrap(), 7);
    let exit = c.next_exit(Duration::from_secs(1)).unwrap().unwrap();
    assert_eq!(
        (
            exit.pid,
            exit.exit_type,
            exit.reason,
            exit.category.as_str()
        ),
        (7, ExitType::Panic, 3, "RUST")
    );
}

#[test]
fn an_error_answer_names_the_method_code_and_message() {
    let (_d, sock) = fake(|l| {
        vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":-32002,\"message\":\"no app with UID 0xE1234567\"}}}}",
            id_of(l)
        )]
    });
    let e = ControlClient::connect(&sock)
        .unwrap()
        .launch(0xe123_4567)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("app.launch") && e.contains("-32002") && e.contains("no app with UID"),
        "{e}"
    );
}

#[test]
fn another_protocol_is_refused_with_what_to_install() {
    let (_d, sock) = fake(|l| {
        vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"name\":\"EKA2L1\",\"version\":\"x\",\"protocol\":2,\"paused\":false,\"device\":null}}}}",
            id_of(l)
        )]
    });
    let e = ControlClient::connect(&sock)
        .unwrap()
        .info()
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("protocol 2") && e.contains("SYMDEV_EKA2L1"),
        "{e}"
    );
}

#[test]
fn a_quiet_socket_is_no_exit_and_a_closed_one_is_the_emulator_gone() {
    let (_d, sock) = fake(|_| Vec::new());
    let mut quiet = ControlClient::connect(&sock).unwrap();
    assert!(matches!(
        quiet.next_exit(Duration::from_millis(200)),
        Ok(None)
    ));

    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("g.sock");
    let listener = UnixListener::bind(&gone).unwrap();
    std::thread::spawn(move || drop(listener.accept().unwrap())); // accept, then hang up
    let mut c = ControlClient::connect(&gone).unwrap();
    let e = c.next_exit(Duration::from_secs(2)).unwrap_err().to_string();
    assert!(
        e.starts_with("the emulator closed its control connection"),
        "{e}"
    );
}

#[test]
fn paths_with_quotes_and_backslashes_are_escaped() {
    let line = super::Request::line(
        1,
        "package.install",
        &[("path", super::Param::Str("/a\"b\\c.sisx"))],
    );
    assert_eq!(
        line,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"package.install\",\"params\":{\"path\":\"/a\\\"b\\\\c.sisx\"}}"
    );
}
