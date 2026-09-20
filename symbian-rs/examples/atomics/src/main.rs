#![no_std]
#![no_main]

extern crate alloc;

use alloc::sync::Arc;
use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};

use symbian_core::{Buf16, user};

static COUNT: AtomicU32 = AtomicU32::new(0);

fn trace(args: core::fmt::Arguments<'_>) {
    let mut note = Buf16::<96>::new();
    let _ = note.write_fmt(args);
    let _ = user::info_print(&note);
}

fn main() -> i32 {
    trace(format_args!("TRACE rust main entered"));
    let status = unsafe { symbian_sys::shim::symrs_atomic_init_status() };
    let handle = unsafe { symbian_sys::shim::symrs_atomic_lock_handle() };
    trace(format_args!("TRACE status={status} handle={handle}"));
    COUNT.store(5, Ordering::SeqCst);
    trace(format_args!("TRACE stored"));
    let loaded = COUNT.load(Ordering::SeqCst);
    trace(format_args!("TRACE loaded={loaded}"));
    let old = COUNT.fetch_add(2, Ordering::SeqCst);
    trace(format_args!("TRACE fetch_add old={old} now={}", COUNT.load(Ordering::SeqCst)));
    let cas = COUNT.compare_exchange(7, 9, Ordering::SeqCst, Ordering::SeqCst);
    trace(format_args!("TRACE cas={cas:?}"));
    let arc = Arc::new(3u32);
    trace(format_args!("TRACE arc made"));
    let second = Arc::clone(&arc);
    trace(format_args!("TRACE arc cloned count={}", Arc::strong_count(&arc)));
    let sum = *arc + *second;
    drop(second);
    trace(format_args!("TRACE sum={sum} count={}", Arc::strong_count(&arc)));
    user::after(1_000_000);
    0
}

symbian_runtime::entry!(main);
