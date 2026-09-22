//! The facilities step 77 left open and the std-gaps slice made real: `sys::path`'s
//! drive prefixes, `read_dir`, `args`, and the half of `process` that exists.
//!
//! Everything here runs inside the emulator and reports through the same `Report` the
//! rest of the example uses, so a regression is a failed case and not a silence.

use std::io::Write as _;
use std::path::{Component, Path, Prefix};

use symbian_std::test_report::{Report, detail};

/// `std::path` over a Symbian path: the drive letter is a `Prefix::Disk` now, so every
/// question `Path` answers about it is the right one.
pub fn path_cases(report: &mut Report) {
    let file = Path::new("E:\\symdev\\std77\\roundtrip.txt");
    report.check("a drive-rooted path is absolute", file.is_absolute());
    report.check(
        "a path with no drive is not absolute",
        Path::new("\\symdev\\x").is_relative(),
    );
    // Symbian keeps **one** session path, not one per drive (`f32file.h`, the `RFs`
    // class documentation), so `E:x` still needs that session path and is relative.
    report.check(
        "E:x is relative, because the session path is what completes it",
        Path::new("E:roundtrip.txt").is_relative(),
    );
    report.check_detail(
        "the drive is a Prefix::Disk",
        matches!(
            file.components().next(),
            Some(Component::Prefix(p)) if p.kind() == Prefix::Disk(b'E')
        ),
        detail!("{:?}", file.components().next()),
    );
    report.check(
        "a lower-case drive is the same drive",
        Path::new("e:\\x").components().next() == Path::new("E:\\x").components().next(),
    );
    report.check(
        "parent walks up to the drive root and stops",
        Path::new("E:\\symdev").parent() == Some(Path::new("E:\\"))
            && Path::new("E:\\").parent().is_none(),
    );
    report.check(
        "file_name is the last component",
        file.file_name() == Some(std::ffi::OsStr::new("roundtrip.txt")),
    );
    report.check(
        "join keeps the drive",
        Path::new("E:\\symdev").join("std77") == Path::new("E:\\symdev\\std77"),
    );
    report.check(
        "joining an absolute path replaces the whole thing, drive and all",
        Path::new("E:\\symdev").join("C:\\other") == Path::new("C:\\other"),
    );
    // `std::path::absolute` needs `RFs::Parse` against the session path, which has not
    // been observed; it says so rather than guessing.
    report.check_detail(
        "path::absolute is Unsupported and says so",
        std::path::absolute("E:x").is_err(),
        detail!("{:?}", std::path::absolute("E:x").err()),
    );
}

/// `std::fs::read_dir` over `RFs::GetDir`: the listing, the entry types and the sizes
/// the file server put in the `CDir`.
pub fn read_dir_cases(report: &mut Report, parent: &str, already_there: &str) {
    // This example owns the directory, so the listing is exactly what it put there:
    // the file the `fs` cases wrote, two more of ours, and one subdirectory.
    //
    // One level at a time, and not `create_dir_all("…\\a\\b")`, because EKA2L1's file
    // server answers `KErrAlreadyExists` where Symbian's answers `KErrPathNotFound`
    // when the *parent* of a new directory is missing, and `create_dir_all` reads that
    // as "it is already there" and stops. See the note in the backlog.
    let dir = Path::new(parent);
    let sub = dir.join("sub");
    let one = dir.join("one.txt");
    let two = dir.join("two.bin");
    let _ = std::fs::create_dir(&sub);
    write_and_flush(&one, b"one");
    write_and_flush(&two, b"twotwo");

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            report.check_detail("read_dir opens the directory", false, detail!("{e}"));
            return;
        }
    };
    let mut detail = String::new();
    let mut names: Vec<String> = Vec::new();
    let mut dirs = 0usize;
    let mut sizes = 0u64;
    for entry in entries {
        match entry {
            Ok(entry) => {
                names.push(entry.file_name().to_string_lossy().into_owned());
                match entry.file_type() {
                    Ok(t) if t.is_dir() => dirs += 1,
                    Ok(_) => {
                        let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        detail.push_str(&format!("{}={len} ", entry.file_name().to_string_lossy()));
                        sizes += len;
                    }
                    Err(_) => {}
                }
            }
            Err(e) => {
                report.check_detail("every entry reads back", false, detail!("{e}"));
                return;
            }
        }
    }
    names.sort();
    report.check_detail(
        "read_dir lists the files and the subdirectory",
        names == ["one.txt", already_there, "sub", "two.bin"],
        detail!("{names:?}"),
    );
    report.check_detail(
        "file_type tells the subdirectory from the files",
        dirs == 1,
        detail!("{dirs} directories"),
    );
    report.check_detail(
        "metadata().len() comes out of the same listing",
        sizes == 58,
        detail!("{sizes} bytes of files, {detail}"),
    );
    report.check(
        "the listing yields neither . nor ..",
        !names.iter().any(|n| n == "." || n == ".."),
    );
    report.check_detail(
        "DirEntry::path is the full path",
        std::fs::read_dir(dir)
            .map(|mut e| e.any(|e| e.map(|e| e.path() == one).unwrap_or(false)))
            .unwrap_or(false),
        detail!("{}", one.display()),
    );
    let missing = std::fs::read_dir("E:\\symdev\\no-such-directory").err();
    report.check_detail(
        "read_dir on a missing directory is an error, not an empty listing",
        matches!(&missing, Some(e) if e.kind() == std::io::ErrorKind::NotFound),
        detail!("{missing:?}"),
    );

    let _ = std::fs::remove_file(&one);
    let _ = std::fs::remove_file(&two);
}

/// `fs::write` and then `sync_all`, because the size a closed file reports under EKA2L1
/// is the size at its last flush: without the `RFile::Flush` both the directory listing
/// and `metadata` answer 0 for a file whose bytes are already on the host's disk. It is
/// the emulator's, not the file server's, and it is worked around here rather than
/// hidden inside `std`.
fn write_and_flush(path: &Path, bytes: &[u8]) {
    if let Ok(mut file) = std::fs::File::create(path) {
        let _ = file.write_all(bytes);
        let _ = file.sync_all();
    }
}

/// `std::env::args` over `User::CommandLine`, and `std::env::var`, which is `Unsupported`
/// on purpose and for good.
pub fn args_and_env_cases(report: &mut Report) {
    let argv: Vec<String> = std::env::args().collect();
    report.check_detail(
        "args() always has a first element",
        !argv.is_empty(),
        detail!("{argv:?}"),
    );
    // Element 0 is `RProcess().FileName()` — Symbian's command line does not carry the
    // program name, so the shim reads the running image's own path instead.
    report.check_detail(
        "args()[0] is this image's own path",
        argv.first()
            .map(|a| a.to_ascii_lowercase().ends_with("stdhello.exe"))
            .unwrap_or(false),
        detail!("{:?}", argv.first()),
    );
    report.check(
        "args()[0] names a drive, so it is an absolute Symbian path",
        argv.first()
            .map(|a| Path::new(a).is_absolute())
            .unwrap_or(false),
    );

    // There is no environment at all on this platform, and that is the answer rather
    // than a panic: `vars()` is empty and `var` says NotPresent.
    report.check_detail(
        "env::vars() is empty and does not panic",
        std::env::vars().count() == 0,
        detail!("{} variables", std::env::vars().count()),
    );
    report.check_detail(
        "env::var is NotPresent, because Symbian has no environment",
        std::env::var("PATH") == Err(std::env::VarError::NotPresent),
        detail!("{:?}", std::env::var("PATH")),
    );
}
