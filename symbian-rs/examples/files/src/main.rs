//! Files, in `std`'s shape (design spec §6a, §11 step 71).
//!
//! The program writes a file, closes it, re-opens it, reads it back and compares —
//! and the only thing in it that is Symbian at all is the path. Every `use` line, every
//! type and every method below is `std`'s: `fs::File`, `io::Write::write_all`,
//! `fs::read`, `io::Error::kind`. That is the point of the slice.
//!
//! It reports through [`symbian_std::test_report`], which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.
#![no_std]
#![no_main]

extern crate alloc;

use symbian_std::fs::{self, File};
use symbian_std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use symbian_std::test_report::Report;


/// Symbian paths: a drive letter and backslashes, which is why the literals are
/// escaped. There is no POSIX root above `E:`.
const DIR: &str = "E:\\symdev\\files71";
const PATH: &str = "E:\\symdev\\files71\\roundtrip.bin";
const RENAMED: &str = "E:\\symdev\\files71\\renamed.bin";
/// Left behind on purpose, so the bytes can be checked from the host afterwards
/// (`~/.local/share/EKA2L1/data/drives/e/symdev/files71/kept.bin`).
const KEPT: &str = "E:\\symdev\\files71\\kept.bin";

/// Something with a non-ASCII byte in it, so the round trip is not just ASCII, and
/// long enough that a truncated write would show.
const BYTES: &[u8] = "symdev step 71 — files through symbian-std.\n".as_bytes();

/// Writes the file and closes it. Closing is what `Drop` does at the end of the scope,
/// so the re-open below really is a second look at what is on the disk.
fn write_it() -> io::Result<()> {
    let mut file = File::create(PATH)?;
    file.write_all(BYTES)?;
    file.sync_all()
}

/// Re-opens the file and reads it in one go, to compare with what went in.
fn read_it() -> io::Result<alloc::vec::Vec<u8>> {
    let mut file = File::open(PATH)?;
    let mut back = alloc::vec::Vec::new();
    file.read_to_end(&mut back)?;
    Ok(back)
}

/// Seeks to the middle of the file and reads three bytes from there, which is the
/// part `read_to_end` alone would not prove.
fn read_from_middle() -> io::Result<[u8; 3]> {
    let mut file = File::open(PATH)?;
    file.seek(SeekFrom::Start(7))?;
    let mut three = [0u8; 3];
    file.read_exact(&mut three)?;
    Ok(three)
}

fn run(report: &mut Report) {
    report.checked("create_dir_all", fs::create_dir_all(DIR));
    // An existing directory is success in `std`, and must be here too.
    report.checked("create_dir_all is idempotent", fs::create_dir_all(DIR));

    report.checked("write and close", write_it());

    if let Some(meta) = report.checked("metadata", fs::metadata(PATH)) {
        report.check(
            "metadata.len is what was written",
            meta.len() == BYTES.len() as u64,
        );
        report.check(
            "metadata says file, not dir",
            meta.is_file() && !meta.is_dir(),
        );
    }

    if let Some(back) = report.checked("read back", read_it()) {
        report.check("the bytes are the bytes", back == BYTES);
    }

    if let Some(three) = report.checked("seek and read_exact", read_from_middle()) {
        report.check(
            "seek landed where it said",
            three == [BYTES[7], BYTES[8], BYTES[9]],
        );
    }

    // `std`'s error vocabulary over Symbian's codes: a missing file is `NotFound` and
    // still carries `KErrNotFound` underneath.
    match File::open("E:\\symdev\\files71\\not-here.bin") {
        Ok(_) => report.fail("opening a missing file fails", "it opened"),
        Err(e) => {
            report.check(
                "a missing file is NotFound",
                e.kind() == ErrorKind::NotFound,
            );
            report.check("and keeps its TInt", e.raw_os_error() == Some(-1));
        }
    }

    // One file is written with the whole-file helper and left in place, so that what
    // landed on the drive can be compared byte for byte from outside the emulator.
    report.checked("fs::write leaves a file behind", fs::write(KEPT, BYTES));

    report.checked("rename", fs::rename(PATH, RENAMED));
    report.checked("remove_file", fs::remove_file(RENAMED));
    report.check(
        "the file is gone",
        fs::metadata(RENAMED).err().map(|e| e.kind()) == Some(ErrorKind::NotFound),
    );
}

fn main() -> i32 {
    let mut report = Report::new("files");
    run(&mut report);
    match report.finish() {
        Ok(true) => 0,
        // A failing run is a non-zero exit as well as a `failed` count in the file, so
        // the two channels agree.
        Ok(false) => 1,
        // The report could not be written at all: nothing will read a result, so say
        // so with the code that stopped it.
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

symbian_runtime::entry!(main);
