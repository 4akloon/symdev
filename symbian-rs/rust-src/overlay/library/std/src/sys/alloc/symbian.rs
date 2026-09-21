//! The process heap: `User::Alloc` / `Free` / `ReAlloc` on the calling thread's heap.
//!
//! This is `symbian_alloc::SymbianHeap` re-hosted (experiment 68). The `no_std` SDK
//! keeps its own copy because a `#![no_std]` program has no `std::alloc::System` to
//! reach for; when a program does have one, this is where it lands and the SDK crate is
//! not linked at all.
//!
//! # Alignment
//!
//! Every `User::Alloc` cell on this ROM's heap was **measured** to be 8-byte aligned
//! (experiment 68: 32 cells of sizes 1…257, low three bits always clear, cell sizes
//! always a multiple of 8, `User::AllocLen` always `4 (mod 8)` — a 4-byte header in
//! front of an 8-aligned payload). `RHeap::Align` is a per-heap field and not an ABI
//! guarantee, so a request for more than 8 is padded by hand and carries the cell's own
//! address in the four bytes below the payload.
//!
//! # One heap behind every thread
//!
//! `User::Alloc` allocates on the heap of the *calling* thread. `sys::thread` gives a
//! spawned thread a heap of its own and then switches it onto the creator's allocator
//! as its first instruction, so a `Box` may cross threads as Rust expects. Serialising
//! concurrent access to that one heap is euser's job, not this module's: an `RHeap`
//! takes its own lock.

use crate::alloc::{GlobalAlloc, Layout, System};
use crate::ptr;

/// The alignment every cell is observed to have.
const MAX_TRUSTED_ALIGN: usize = 8;

/// `TInt` is signed: a request that does not fit is a failure, not a truncation.
fn cell_size(bytes: usize) -> Option<i32> {
    i32::try_from(bytes).ok()
}

/// Where the aligned payload goes inside an over-aligned cell, and where the cell's own
/// address is remembered so `dealloc` can give euser back what it handed out.
///
/// # Safety
/// `cell` must be a live cell of at least `size + align` bytes from `User::Alloc`, and
/// `align` must be a power of two greater than [`MAX_TRUSTED_ALIGN`].
unsafe fn place_padded(cell: *mut u8, align: usize) -> *mut u8 {
    let payload = (cell.addr() + align & !(align - 1)) as *mut u8;
    // SAFETY: the cell is 8-aligned and `align > 8`, so the aligned address is between
    // 8 and `align` bytes above the cell and `payload - 4` is inside it and 4-aligned.
    // The slot is written before any caller sees the payload.
    unsafe { payload.cast::<*mut u8>().sub(1).write(cell) };
    payload
}

/// The cell that [`place_padded`] was given.
///
/// # Safety
/// `payload` must have come from [`place_padded`].
unsafe fn padded_cell(payload: *mut u8) -> *mut u8 {
    // SAFETY: the back pointer was written just below the payload by `place_padded`.
    unsafe { payload.cast::<*mut u8>().sub(1).read() }
}

#[inline]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    if layout.align() <= MAX_TRUSTED_ALIGN {
        return match cell_size(layout.size()) {
            // SAFETY: a plain heap request; euser returns null on failure and takes
            // ownership of nothing.
            Some(n) => unsafe { symbian_sys::euser::User_Alloc(n) },
            None => ptr::null_mut(),
        };
    }
    let Some(n) = layout.size().checked_add(layout.align()).and_then(cell_size) else {
        return ptr::null_mut();
    };
    // SAFETY: as above; the cell is large enough for the payload, the alignment slack
    // and the back pointer.
    let cell = unsafe { symbian_sys::euser::User_Alloc(n) };
    if cell.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: `cell` is a live cell of `size + align` bytes and the alignment is over 8.
    unsafe { place_padded(cell, layout.align()) }
}

#[inline]
pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    if layout.align() > MAX_TRUSTED_ALIGN {
        // SAFETY: the padded path allocates and then zeroes what the caller can see.
        let p = unsafe { alloc(layout) };
        if !p.is_null() {
            // SAFETY: `p` is a fresh allocation of `layout.size()` bytes.
            unsafe { ptr::write_bytes(p, 0, layout.size()) };
        }
        return p;
    }
    match cell_size(layout.size()) {
        // SAFETY: `User::AllocZ` is `User::Alloc` with the cell zero-filled.
        Some(n) => unsafe { symbian_sys::euser::User_AllocZ(n) },
        None => ptr::null_mut(),
    }
}

#[inline]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    let cell = if layout.align() > MAX_TRUSTED_ALIGN {
        // SAFETY: an over-aligned payload always carries its cell address below it.
        unsafe { padded_cell(ptr) }
    } else {
        ptr
    };
    // SAFETY: `cell` is exactly what `User::Alloc` returned for this allocation, and it
    // is freed once.
    unsafe { symbian_sys::euser::User_Free(cell) };
}

#[inline]
pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    if layout.align() > MAX_TRUSTED_ALIGN {
        // `User::ReAlloc` may move the cell and promises nothing beyond the heap's own
        // alignment, so an over-aligned block is re-placed by hand.
        // SAFETY: the new layout has the same (valid) alignment.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: `System` is this module; the call is the same one the caller made.
        let new = unsafe { System.alloc(new_layout) };
        if !new.is_null() {
            // SAFETY: both blocks are live and do not overlap; only the bytes that
            // exist in both are copied.
            unsafe { ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
            // SAFETY: the old block is still the one this layout describes.
            unsafe { System.dealloc(ptr, layout) };
        }
        return new;
    }
    match cell_size(new_size) {
        // SAFETY: mode 0 is euser's default `RAllocator::ReAlloc` mode: the cell may
        // move, the contents up to the smaller of the two sizes are preserved, and null
        // is returned with the old cell untouched on failure (observed by
        // `symbian-rs/examples/alloc`, which grows a `Vec` through this path).
        Some(n) => unsafe { symbian_sys::euser::User_ReAlloc(ptr, n, 0) },
        None => ptr::null_mut(),
    }
}
