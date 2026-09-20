//! The `alloc` example (experiment 68): the Rust heap on the Symbian heap.
//!
//! It grows a `Vec<u16>` and a `String` through `User::ReAlloc`, builds a heap-backed
//! descriptor, allocates an over-aligned value through the allocator's padding path, and
//! prints numbers derived from the data so the log proves the round trip happened. No
//! `unsafe` anywhere: everything goes through `symbian-core` and the global allocator.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use symbian_core::{Buf16, Des16, DesC16, ErrorKind, HBuf16, Result, SymbianError, user};

/// A value the Symbian heap cannot align on its own: 32 bytes, 32-aligned. The global
/// allocator has to over-allocate and place it by hand (`MAX_TRUSTED_ALIGN` is 8).
#[repr(align(32))]
struct Over([u8; 32]);

fn overflow() -> SymbianError {
    SymbianError::of(ErrorKind::Overflow)
}

fn run() -> Result<()> {
    // A `Vec` that outgrows its cell several times: every growth is a `User::ReAlloc`.
    let mut squares: Vec<u16> = Vec::new();
    for i in 0..64u16 {
        squares.push(i.wrapping_mul(i));
    }
    let sum: u32 = squares.iter().map(|u| u32::from(*u)).sum();
    let cells_aligned = squares.as_ptr() as usize % 8;

    // A `String` built by formatting, so the UTF-8 side of the heap is exercised too.
    let mut text = String::new();
    for u in squares.iter().take(5) {
        if write!(text, "{u},").is_err() {
            return Err(overflow());
        }
    }

    // A heap-backed descriptor: one cell holding the header word and the code units.
    let mut heap = HBuf16::from_str("heap ")?;
    heap.push_str(&text)?;
    user::info_print(&heap)?;
    user::after(400_000);

    // The over-aligned path.
    let over = Box::new(Over([0xa5; 32]));
    let over_remainder = (&raw const *over) as usize % 32;
    let over_byte = over.0[31];

    let mut msg = Buf16::<160>::new();
    if write!(
        msg,
        "alloc sum={sum} cap={} a8={cells_aligned} a32={over_remainder} byte={over_byte:x} heap={}",
        squares.capacity(),
        heap.len(),
    )
    .is_err()
    {
        return Err(overflow());
    }

    // Everything on the heap dies before the message is shown: the numbers in it came
    // out of the data while it was alive.
    drop(squares);
    drop(text);
    drop(over);
    drop(heap);

    user::info_print(&Des16::of(&msg))?;
    user::after(2_000_000);
    Ok(())
}

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

symbian_runtime::entry!(main);
