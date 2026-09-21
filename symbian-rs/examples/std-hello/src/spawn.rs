//! The `std::process` cases: what `RProcess` can carry into a child and what it
//! cannot.

use symbian_std::test_report::Report;

/// The exit code the child is asked for, which is also the argument it is given — so
/// one number proves the command line went out and the exit code came back.
const CHILD_CODE: i32 = 7;

/// The image the child cases spawn: `examples/spawnee`, which must be installed first
/// (`cd ../spawnee && symdev build && symdev package && symdev run`), the same way
/// `examples/net` needs its two peers running.
///
/// It is a separate, `no_std` image and not this one because **EKA2L1 cannot spawn an
/// image with a writable data section through the loader**: `RProcess::Create`
/// succeeds, the emulator gives the child an `anonymous` 0x1000-byte data chunk, and
/// the child dies with `KERN-EXEC 3` before reaching `main` — after which the parent
/// waits on a `Logon` that never completes. Every `std` image tried does this and no
/// `no_std` one does. See `examples/spawnee`.
///
/// `RProcess::Create` takes a file name and not a search path, so it is named in full.
const CHILD_IMAGE: &str = "E:\\sys\\bin\\spawnee.exe";

/// Where the child writes the command line it was given.
const CHILD_MARK: &str = "E:\\symdev\\spawnee\\args.txt";

/// `std::process`: spawn, wait and an exit code are real; stdio redirection is not and
/// says so.
pub fn process_cases(report: &mut Report) {
    use std::process::{Command, Stdio};

    report.check_detail(
        "process::id is this process's own",
        std::process::id() != 0,
        format_args!("{}", std::process::id()),
    );

    // The child is a separate image given one argument, which is also the exit code
    // it uses: one number goes out through `RProcess::Create`'s command line and comes
    // back through `RProcess::Logon`.
    let _ = std::fs::remove_file(CHILD_MARK);
    match Command::new(CHILD_IMAGE)
        .arg(CHILD_CODE.to_string())
        .spawn()
    {
        Ok(mut child) => {
            report.check_detail(
                "RProcess::Create + Resume spawn examples/spawnee",
                true,
                format_args!("pid {}", child.id()),
            );
            match child.wait() {
                Ok(status) => report.check_detail(
                    "RProcess::Logon gives the child's exit code back",
                    status.code() == Some(CHILD_CODE),
                    format_args!("{status}"),
                ),
                Err(e) => report.check_detail(
                    "RProcess::Logon gives the child's exit code back",
                    false,
                    format_args!("{e}"),
                ),
            }
            let mark = std::fs::read_to_string(CHILD_MARK).unwrap_or_default();
            report.check_detail(
                "the command line reached the child verbatim",
                mark.trim() == CHILD_CODE.to_string(),
                format_args!("{mark}"),
            );
        }
        Err(e) => report.check_detail(
            "RProcess::Create + Resume spawn examples/spawnee",
            false,
            format_args!("{e} — is examples/spawnee installed?"),
        ),
    }

    report.check_detail(
        "a missing image is an error, not a spawn",
        Command::new("E:\\sys\\bin\\no-such-program.exe")
            .spawn()
            .is_err(),
        format_args!(
            "{:?}",
            Command::new("E:\\sys\\bin\\no-such-program.exe")
                .spawn()
                .err()
        ),
    );

    // What this platform cannot carry into a child, refused at the spawn rather than
    // silently dropped.
    let unsupported =
        |e: Option<std::io::Error>| e.map(|e| e.kind()) == Some(std::io::ErrorKind::Unsupported);
    report.check(
        "Stdio::piped() is Unsupported: there is no RPipe in this SDK",
        unsupported(
            Command::new(CHILD_IMAGE)
                .stdout(Stdio::piped())
                .spawn()
                .err(),
        ),
    );
    report.check(
        "output() is Unsupported for the same reason",
        unsupported(Command::new(CHILD_IMAGE).output().err()),
    );
    report.check(
        "current_dir is Unsupported: the session path is per session, not per process",
        unsupported(Command::new(CHILD_IMAGE).current_dir("E:\\").spawn().err()),
    );
    report.check(
        "env() is Unsupported: Symbian has no environment to inherit",
        unsupported(Command::new(CHILD_IMAGE).env("SYMDEV", "1").spawn().err()),
    );
}
