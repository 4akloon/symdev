//! Hello: the experiment-65 application. Shows a note through `User::InfoPrint`, waits
//! five seconds so it can be seen, and exits 0.
#![no_std]
#![no_main]

use symbian_runtime::symbian_sys::des::Lit16;
use symbian_runtime::symbian_sys::euser::{User_After, User_InfoPrint};

static HELLO: Lit16<19> = Lit16::ascii(b"Hello from Rust SDK");

fn main() {
    // SAFETY: `HELLO` has the observed `_LIT16` layout and lives for the whole process.
    unsafe {
        User_InfoPrint(HELLO.as_desc());
        User_After(5_000_000);
    }
}

symbian_runtime::entry!(main);
