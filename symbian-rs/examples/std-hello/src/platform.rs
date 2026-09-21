//! The facilities step 77 left open and the std-gaps slice made real: `sys::path`'s
//! drive prefixes, `read_dir`, `args`, and the half of `process` that exists.
//!
//! Everything here runs inside the emulator and reports through the same `Report` the
//! rest of the example uses, so a regression is a failed case and not a silence.

use std::path::{Component, Path, Prefix};

use symbian_std::test_report::Report;

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
        format_args!("{:?}", file.components().next()),
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
        format_args!("{:?}", std::path::absolute("E:x").err()),
    );
}
