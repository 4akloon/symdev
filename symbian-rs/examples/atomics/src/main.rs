#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};

use symbian_core::{Buf16, DesC16, user};
use symbian_sys::thread as sys;

static COUNT: AtomicU32 = AtomicU32::new(0);

const ITERATIONS: u32 = 500;

fn trace(args: core::fmt::Arguments<'_>) {
    let mut note = Buf16::<96>::new();
    let _ = note.write_fmt(args);
    let _ = user::info_print(&note);
}

fn alloc_probe(label: &str) {
    let mut v: Vec<u32> = Vec::new();
    for i in 0..32u32 {
        v.push(i);
    }
    trace(format_args!("TRACE alloc {label} sum={}", v.iter().sum::<u32>()));
}

fn raw_alloc_probe(label: &str) {
    // SAFETY: User::Alloc/Free are euser statics; the cell is used and freed here.
    let cell = unsafe { symbian_sys::euser::User_Alloc(64) };
    trace(format_args!("TRACE User::Alloc {label} ptr={:#x}", cell as usize));
    if !cell.is_null() {
        unsafe { symbian_sys::euser::User_Free(cell) };
    }
}

fn hammer() {
    for _ in 0..ITERATIONS {
        COUNT.fetch_add(1, Ordering::SeqCst);
        user::after(0);
    }
}

unsafe extern "C" fn worker(_: *mut c_void) -> i32 {
    hammer();
    0
}

fn main() -> i32 {
    trace(format_args!("TRACE main entered"));
    alloc_probe("before create");
    raw_alloc_probe("before create");

    let mut thread = sys::RThread::null();
    let mut name = Buf16::<32>::new();
    let _ = name.write_str("symrs-worker");
    let heap = unsafe { sys::User_Allocator() };
    trace(format_args!("TRACE allocator={:#x}", heap as usize));
    let rc = unsafe {
        sys::RThread_CreateWithOwnHeap(
            &mut thread,
            name.as_tdesc16(),
            worker,
            sys::KDEFAULT_STACK_SIZE,
            0x1000,
            0x10000,
            core::ptr::null_mut(),
            sys::EOWNER_PROCESS,
        )
    };
    trace(format_args!("TRACE create rc={rc}"));
    if rc != 0 {
        return rc;
    }
    alloc_probe("after create, before resume");

    let mut status = sys::TRequestStatus::new();
    unsafe {
        sys::RThread_Logon(&thread, &mut status);
        sys::RThread_Resume(&thread);
    }
    user::after(1000);
    alloc_probe("while the worker runs");
    raw_alloc_probe("while the worker runs");
    hammer();
    unsafe { sys::User_WaitForRequest(&mut status) };
    trace(format_args!(
        "TRACE joined status={} count={}",
        status.status,
        COUNT.load(Ordering::SeqCst)
    ));
    raw_alloc_probe("after join, before Close");
    unsafe { symbian_sys::euser::RHandleBase_Close(&mut thread.base) };
    trace(format_args!("TRACE closed the handle"));
    raw_alloc_probe("after Close");
    alloc_probe("after Close");
    trace(format_args!("TRACE still alive"));
    0
}

symbian_runtime::entry!(main);
