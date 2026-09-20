//! Hello, raw: the experiment-65 application as it was before `symbian-core` existed —
//! a `_LIT16` static and two `unsafe` euser calls. Kept as the regression that shows the
//! raw path still works and what the safe version in `examples/hello` costs.
#![no_std]

use symbian_runtime::symbian_sys::des::Lit16;
use symbian_runtime::symbian_sys::euser::{User_After, User_InfoPrint};

static HELLO: Lit16<19> = Lit16::ascii(b"Hello from Rust SDK");

#[symbian_std::main]
fn main() {
    // SAFETY: `HELLO` has the observed `_LIT16` layout and lives for the whole process.
    unsafe {
        User_InfoPrint(HELLO.as_desc());
        User_After(5_000_000);
    }
}
