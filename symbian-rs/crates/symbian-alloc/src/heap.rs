//! `SymbianHeap`: the current thread's Symbian heap behind Rust's `GlobalAlloc`.
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use symbian_sys::euser::{User_Alloc, User_AllocZ, User_Exit, User_Free, User_ReAlloc};

/// `KErrNoMemory` (`e32err.h`).
pub const KERR_NO_MEMORY: i32 = -4;

/// The alignment every `User::Alloc` cell is **observed** to have (experiment 68): on the
/// S60 3rd FP2 ROM in EKA2L1, 32 cells of sizes 1…257 all came back with the low three
/// bits clear, cell sizes were always multiples of 8 and `User::AllocLen` always returned
/// `4 (mod 8)` — a 4-byte cell header in front of an 8-aligned payload. `RHeap::Align`
/// uses a per-heap `iAlign` field, so this is a property of this thread's heap, not an
/// ABI guarantee; a request for more than this is padded by hand below rather than
/// trusted, and a request for this or less is passed straight through.
pub const MAX_TRUSTED_ALIGN: usize = 8;

/// Ends the process the Symbian way when an infallible allocation fails.
///
/// Rust's out-of-memory path must not unwind (`panic = "abort"`, design spec §3) and must
/// not become a Rust panic, whose exit code says nothing: `User::Exit(KErrNoMemory)` is
/// what a Symbian program does, and the loader reports `-4`.
pub fn oom() -> ! {
    // SAFETY: `User::Exit` is a static member function of euser (plain EABI, no `this`),
    // takes ownership of nothing, never returns and is callable from any thread.
    unsafe { User_Exit(KERR_NO_MEMORY) }
}

/// The global heap of the *current thread*.
///
/// `User::Alloc` allocates on the heap of the thread that calls it. A second thread made
/// with `RThread::Create` gets its own heap unless it is told to share one, so a pointer
/// allocated here may only be freed on the thread that allocated it, and `Send`ing an
/// allocation to another thread would free it on the wrong heap. Nothing in this crate
/// creates a thread; when threads arrive (step 72) the allocator must be revisited, not
/// the callers.
pub struct SymbianHeap;

impl SymbianHeap {
    /// `TInt` is signed: a request that does not fit is a failure, not a truncation.
    fn cell_size(bytes: usize) -> Option<i32> {
        i32::try_from(bytes).ok()
    }

    /// Where the aligned payload goes inside an over-aligned cell, and where the cell's
    /// own address is remembered so `dealloc` can give euser back what it handed out.
    ///
    /// The cell is 8-aligned and `align` is a power of two greater than 8, so the aligned
    /// address is at least 8 bytes above the cell and there is always room for the
    /// 4-byte back pointer just below it.
    ///
    /// # Safety
    /// `cell` must be a live cell of at least `size + align` bytes from `User::Alloc`.
    unsafe fn place_padded(cell: *mut u8, align: usize) -> *mut u8 {
        let aligned = (cell as usize + align) & !(align - 1);
        let payload = aligned as *mut u8;
        // SAFETY: `aligned - cell` is in `[8, align]`, so `payload - 4` is inside the
        // cell and 4-aligned; the slot is written before any caller sees the payload.
        unsafe { payload.cast::<*mut u8>().sub(1).write(cell) };
        payload
    }

    /// The cell that [`place_padded`](Self::place_padded) was given.
    ///
    /// # Safety
    /// `payload` must have come from `place_padded`.
    unsafe fn padded_cell(payload: *mut u8) -> *mut u8 {
        // SAFETY: the back pointer was written just below the payload by `place_padded`.
        unsafe { payload.cast::<*mut u8>().sub(1).read() }
    }
}

// SAFETY: every pointer returned is either a `User::Alloc` cell (alignment 8, observed)
// or a hand-aligned address inside one, sized as the layout asks; `dealloc` gives euser
// back exactly the cell it returned; the heap is the calling thread's, and this allocator
// is only ever reached from the thread that owns it (single-threaded until step 72).
unsafe impl GlobalAlloc for SymbianHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MAX_TRUSTED_ALIGN {
            return match Self::cell_size(layout.size()) {
                // SAFETY: a plain heap request; euser returns null on failure.
                Some(n) => unsafe { User_Alloc(n) },
                None => ptr::null_mut(),
            };
        }
        let Some(n) = layout
            .size()
            .checked_add(layout.align())
            .and_then(Self::cell_size)
        else {
            return ptr::null_mut();
        };
        // SAFETY: as above; the cell is large enough for the payload plus the alignment
        // slack and the back pointer.
        let cell = unsafe { User_Alloc(n) };
        if cell.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: `cell` is a live cell of `size + align` bytes.
        unsafe { Self::place_padded(cell, layout.align()) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.align() > MAX_TRUSTED_ALIGN {
            // SAFETY: the padded path allocates and then zeroes what the caller can see.
            let p = unsafe { self.alloc(layout) };
            if !p.is_null() {
                // SAFETY: `p` is a fresh allocation of `layout.size()` bytes.
                unsafe { ptr::write_bytes(p, 0, layout.size()) };
            }
            return p;
        }
        match Self::cell_size(layout.size()) {
            // SAFETY: `User::AllocZ` is `User::Alloc` with the cell zero-filled.
            Some(n) => unsafe { User_AllocZ(n) },
            None => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let cell = if layout.align() > MAX_TRUSTED_ALIGN {
            // SAFETY: an over-aligned payload always carries its cell address below it.
            unsafe { Self::padded_cell(ptr) }
        } else {
            ptr
        };
        // SAFETY: `cell` is exactly what `User::Alloc` returned for this allocation and
        // is freed once, on the thread that allocated it.
        unsafe { User_Free(cell) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() > MAX_TRUSTED_ALIGN {
            // `User::ReAlloc` may move the cell, and it promises nothing beyond the
            // heap's own alignment, so an over-aligned block is re-placed by hand.
            // SAFETY: the new layout has the same (valid) alignment and a non-zero size.
            let new = unsafe { self.alloc(Layout::from_size_align_unchecked(new_size, layout.align())) };
            if !new.is_null() {
                // SAFETY: both blocks are live and do not overlap; only the bytes that
                // exist in both are copied.
                unsafe { ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
                // SAFETY: the old block is still the one this layout describes.
                unsafe { self.dealloc(ptr, layout) };
            }
            return new;
        }
        match Self::cell_size(new_size) {
            // SAFETY: mode 0 is euser's default `RAllocator::ReAlloc` mode: the cell may
            // move, the contents up to the smaller of the two sizes are preserved, and
            // null is returned with the old cell untouched on failure (observed by
            // `examples/alloc`, which grows a `Vec` through this path and reads it back).
            Some(n) => unsafe { User_ReAlloc(ptr, n, 0) },
            None => ptr::null_mut(),
        }
    }
}
