//! The step-77 milestone: **no `#![no_std]`**.
//!
//! This file's first three lines are `use std::fs::File;`, `use std::io::...` and
//! `use std::time::...`. There is no `#![no_std]`, no `extern crate alloc`, no
//! `#[global_allocator]`, no `#[panic_handler]`: a real `std` for
//! `target_os = "symbian"` supplies all of them (design spec §11 step 77).
//!
//! What is still Symbian, because it must be: the path names a drive and separates
//! with backslashes, and the entry point is `#[symbian_std::main]`, because the image's
//! entry is the C++-mangled `E32Main()` that `eexe.lib` calls and rustc never looks for
//! a `fn main` in a `staticlib`.
//!
//! It also depends on two crates from crates.io — `itoa` and `ryu` — with nothing
//! vendored, to show that an ordinary `std`-only dependency compiles for this target.
//!
//! It reports through `symbian_std::test_report`, which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use symbian_std::test_report::Report;

/// Symbian paths: a drive letter and backslashes. There is no POSIX root above `E:`.
const DIR: &str = "E:\\symdev\\std77";
const PATH: &str = "E:\\symdev\\std77\\roundtrip.txt";

const BYTES: &[u8] = "symdev step 77 — std::fs::File with no no_std.\n".as_bytes();

#[symbian_std::main]
fn main() -> std::io::Result<()> {
    let mut report = Report::new("stdhello");

    // The heap, through `std`'s own global allocator: `String`, `Vec`, `format!`.
    let greeting = format!("hello from std, {} bytes to write", BYTES.len());
    report.check("format! on std's allocator", greeting.ends_with("to write"));
    let mut owned: Vec<u8> = Vec::new();
    owned.extend_from_slice(BYTES);
    report.check("Vec grows", owned.len() == BYTES.len());

    // The milestone: a file written and read back through `std::fs`.
    fs::create_dir_all(DIR)?;
    report.check("create_dir_all", fs::exists(DIR).unwrap_or(false));

    {
        let mut file = File::create(PATH)?;
        file.write_all(BYTES)?;
        file.sync_all()?;
    }
    report.check("File::create + write_all", true);

    let read_back = fs::read(PATH)?;
    report.check_detail(
        "fs::read gives back what was written",
        read_back == BYTES,
        format_args!("{} bytes", read_back.len()),
    );

    let metadata = fs::metadata(PATH)?;
    report.check_detail(
        "metadata().len()",
        metadata.len() == BYTES.len() as u64,
        format_args!("{}", metadata.len()),
    );
    report.check("metadata().is_file()", metadata.is_file());

    // Seeking and partial reads, so the handle is more than an open-and-slurp.
    let mut file = File::open(PATH)?;
    file.seek(SeekFrom::Start(7))?;
    let mut seven = [0u8; 6];
    file.read_exact(&mut seven)?;
    report.check_detail(
        "seek then read_exact",
        &seven == b"step 7",
        format_args!("{}", String::from_utf8_lossy(&seven)),
    );
    drop(file);

    // An error that keeps its `ErrorKind` and its Symbian code.
    let missing = File::open("E:\\symdev\\std77\\nothing-here.txt").unwrap_err();
    report.check_detail(
        "a missing file is NotFound",
        missing.kind() == ErrorKind::NotFound,
        format_args!("{missing}"),
    );
    report.check_detail(
        "the TInt survives in raw_os_error",
        missing.raw_os_error() == Some(-1),
        format_args!("{:?}", missing.raw_os_error()),
    );

    // `println!` goes to `E:\symdev\stdout.txt`, not nowhere.
    println!("{greeting}");
    println!("this line is in E:\\symdev\\stdout.txt");

    // `std::time`, both clocks.
    let started = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(120));
    let elapsed = started.elapsed();
    report.check_detail(
        "Instant measures a sleep",
        elapsed >= std::time::Duration::from_millis(80),
        format_args!("{} ms", elapsed.as_millis()),
    );
    let wall = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    report.check_detail(
        "SystemTime is after 2010",
        wall.as_secs() > 1_262_304_000,
        format_args!("{} s since the epoch", wall.as_secs()),
    );

    // A `thread_local!`, whose destructor `std::os::symbian::start` has to run because
    // nothing in the kernel will.
    thread_local! {
        static COUNTER: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    COUNTER.with(|c| c.set(c.get() + 41));
    report.check("thread_local!", COUNTER.with(|c| c.get()) == 41);

    // A spawned thread sharing the one heap.
    let worker = std::thread::spawn(|| {
        let mut sum = 0u32;
        for i in 1..=100 {
            sum += i;
        }
        format!("{sum}")
    });
    let from_worker = worker.join().unwrap_or_else(|_| String::from("panicked"));
    report.check_detail("thread::spawn + join", from_worker == "5050", format_args!("{from_worker}"));

    // A `Mutex` and an `Arc`, which is what `sys::sync` is for.
    let shared = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let other = std::sync::Arc::clone(&shared);
    let bumper = std::thread::spawn(move || {
        for _ in 0..1000 {
            if let Ok(mut n) = other.lock() {
                *n += 1;
            }
        }
    });
    for _ in 0..1000 {
        if let Ok(mut n) = shared.lock() {
            *n += 1;
        }
    }
    let _ = bumper.join();
    let total = shared.lock().map(|n| *n).unwrap_or(0);
    report.check_detail("two threads through a Mutex", total == 2000, format_args!("{total}"));

    // Two crates from crates.io, compiled unchanged for this target.
    let mut itoa_buf = itoa::Buffer::new();
    let itoa_text = itoa_buf.format(-4242).to_owned();
    report.check_detail("itoa from crates.io", itoa_text == "-4242", format_args!("{itoa_text}"));
    let mut ryu_buf = ryu::Buffer::new();
    let ryu_text = ryu_buf.format(1.5f64).to_owned();
    report.check_detail("ryu from crates.io", ryu_text == "1.5", format_args!("{ryu_text}"));

    // A `BTreeMap` and a `HashMap`, the second of which needs `sys::random`.
    let mut sorted = std::collections::BTreeMap::new();
    sorted.insert("b", 2);
    sorted.insert("a", 1);
    report.check("BTreeMap orders its keys", sorted.keys().copied().eq(["a", "b"]));
    let mut hashed = std::collections::HashMap::new();
    hashed.insert("answer", 42);
    report.check("HashMap needs sys::random", hashed.get("answer") == Some(&42));

    fs::remove_file(PATH)?;
    report.check("remove_file", !fs::exists(PATH).unwrap_or(true));

    report.finish()?;
    Ok(())
}
