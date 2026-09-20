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
    let status = unsafe { symbian_sys::libcalls::symrs_atomic_init_status() };
    let handle = unsafe { symbian_sys::libcalls::symrs_atomic_lock_handle() };
    trace(format_args!("TRACE status={status} handle={handle}"));
    COUNT.store(5, Ordering::SeqCst);
    let old = COUNT.fetch_add(2, Ordering::SeqCst);
    let cas = COUNT.compare_exchange(7, 9, Ordering::SeqCst, Ordering::SeqCst);
    let miss = COUNT.compare_exchange(7, 11, Ordering::SeqCst, Ordering::SeqCst);
    let arc = Arc::new(3u32);
    let second = Arc::clone(&arc);
    let count = Arc::strong_count(&arc);
    drop(second);
    trace(format_args!(
        "TRACE old={old} cas={cas:?} miss={miss:?} now={} arc={count}/{}",
        COUNT.load(Ordering::SeqCst),
        Arc::strong_count(&arc)
    ));
    let after = unsafe { symbian_sys::libcalls::symrs_atomic_init_status() };
    let handle2 = unsafe { symbian_sys::libcalls::symrs_atomic_lock_handle() };
    trace(format_args!("TRACE after status={after} handle={handle2}"));
    user::after(500_000);
    0
}

symbian_runtime::entry!(main);
