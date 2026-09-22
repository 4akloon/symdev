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

extern crate alloc;

use symbian_std::fs::{self, File};
use symbian_std::io::{self, ErrorKind, SeekFrom};
use symbian_std::prelude::*;
use symbian_std::test_report::{Report, detail};

/// Symbian paths: a drive letter and backslashes, which is why the literals are
/// escaped. There is no POSIX root above `E:`.
const DIR: &str = "E:\\symdev\\files71";
const PATH: &str = "E:\\symdev\\files71\\roundtrip.bin";
const RENAMED: &str = "E:\\symdev\\files71\\renamed.bin";
/// A directory inside [`DIR`], so the listing has something that is not a file.
const SUB: &str = "E:\\symdev\\files71\\sub";
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

/// What `read_dir` holds on the heap, and whether it gives all of it back.
///
/// Nothing on the Rust side of `read_dir` touches the heap — the path and the pattern
/// are built on the stack — so every cell counted here is one `RFs::GetDir` allocated
/// inside efsrv, and a C++ program making the same call pays the same. What *can* be
/// proved from here is the leak property: once the first read has grown whatever the
/// cleanup stack grows on first use, a second read must leave the count where it was.
fn heap_cost_of_reading(report: &mut Report) {
    let cells = symbian_std::test_report::alloc_cells;
    let before_first = cells();
    let held_first = fs::read_dir(DIR).map(|_dir| cells());
    let after_first = cells();
    let held_second = fs::read_dir(DIR).map(|_dir| cells());
    let after_second = cells();
    let (Ok(held_first), Ok(held_second)) = (held_first, held_second) else {
        report.fail("read_dir twice, counting cells", "a read failed");
        return;
    };
    report.check_detail(
        "a second read_dir leaves no cell behind",
        after_second == after_first,
        detail!(
            "first: {before_first} -> held {held_first} -> {after_first}; \
             second: held {held_second} -> {after_second}"
        ),
    );
}

/// `read_dir` over the directory the cases above filled: `roundtrip.bin`, `kept.bin`
/// and the `sub` directory made here. Every run leaves exactly those three, so the
/// listing is checked for exactly them.
fn list_the_directory(report: &mut Report) {
    report.checked("create a subdirectory", fs::create_dir_all(SUB));
    heap_cost_of_reading(report);
    let Some(dir) = report.checked("read_dir", fs::read_dir(DIR)) else {
        return;
    };

    // The C++ parity claim, measured: the one `CDir` the file server filled is the
    // only heap cell listing needs. Walking every entry, comparing every name and
    // reading every size must leave the thread's cell count exactly where it was.
    let before = symbian_std::test_report::alloc_cells();
    let (mut count, mut dots, mut file, mut kept, mut sub) = (0, 0, false, false, false);
    for entry in &dir {
        count += 1;
        if entry.name() == "." || entry.name() == ".." {
            dots += 1;
        } else if entry.name() == "roundtrip.bin" {
            file = entry.is_file() && entry.size() == BYTES.len() as u64;
        } else if entry.name() == "kept.bin" {
            kept = entry.is_file();
        } else if entry.name() == "sub" {
            sub = entry.is_dir() && !entry.is_file();
        }
    }
    let after = symbian_std::test_report::alloc_cells();

    report.check("read_dir yields no . or ..", dots == 0);
    report.check("the written file is listed, as a file of its size", file);
    report.check("the kept file is listed", kept);
    report.check("the subdirectory is listed as a directory", sub);
    report.check_detail("and nothing else", count == 3, detail!("{count} entries"));
    report.check_detail(
        "listing allocates no heap cell",
        after == before,
        detail!("cells {before} -> {after}"),
    );

    // `KErrPathNotFound` (-12) underneath, `NotFound` on top — the same pair `File::open`
    // gives for a missing file, checked as values rather than printed.
    match fs::read_dir("E:\\symdev\\files71\\no-such-dir") {
        Ok(_) => report.fail("read_dir of a missing directory fails", "it listed"),
        Err(e) => report.check(
            "read_dir of a missing directory is NotFound, KErrPathNotFound",
            e.kind() == ErrorKind::NotFound && e.raw_os_error() == Some(-12),
        ),
    }
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

    list_the_directory(report);

    report.checked("rename", fs::rename(PATH, RENAMED));
    report.checked("remove_file", fs::remove_file(RENAMED));
    report.check(
        "the file is gone",
        fs::metadata(RENAMED).err().map(|e| e.kind()) == Some(ErrorKind::NotFound),
    );
}

/// The report could not be written at all is the `Err`: nothing will read a result,
/// so the process ends with the `TInt` that stopped it. A run that wrote a report
/// exits 0 or 1, so that the exit code and the `failed` count in the file agree.
#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("files");
    run(&mut report);
    Ok(if report.finish()? { 0 } else { 1 })
}
