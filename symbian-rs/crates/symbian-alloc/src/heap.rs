//! `SymbianHeap`: the current thread's Symbian heap behind Rust's `GlobalAlloc`.
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use symbian_sys::euser::{User_Alloc, User_AllocZ, User_Free, User_ReAlloc};

use crate::serialise::HeapGuard;

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

/// The global heap of the *current thread*.
///
/// `User::Alloc` allocates on the heap of the thread that calls it, so "one heap for the
/// whole program" is something the program has to arrange. `symbian_std::thread::spawn`
/// arranges it: a spawned thread is created with a heap of its own and switches to the
/// creating thread's heap as its first instruction (`User::SwitchAllocator`), because
/// passing the creator's allocator to `RThread::Create` instead makes the worker's exit
/// take the creator's heap down with it — the access violation of experiment 80. With
/// one heap behind every thread, a `Box` or an `Arc` may cross threads as Rust expects.
///
/// Concurrent access to that one heap is serialised by [`crate::serialise`], which
/// `spawn` switches on before it creates the first thread. Until then every operation
/// here reads one `TInt` and takes no lock.
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

/// The heap operations themselves, with **no lock taken**.
///
/// They call one another — `alloc_zeroed` over `alloc`, `realloc` over both — and
/// `RFastLock` is not recursive, so the lock is taken exactly once, by the `GlobalAlloc`
/// method the caller entered through.
///
/// # Safety
/// Each has the corresponding `GlobalAlloc` method's contract, and additionally the
/// heap's lock must already be held if serialisation is on.
impl SymbianHeap {
    unsafe fn alloc_unlocked(&self, layout: Layout) -> *mut u8 {
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

    unsafe fn alloc_zeroed_unlocked(&self, layout: Layout) -> *mut u8 {
        if layout.align() > MAX_TRUSTED_ALIGN {
            // SAFETY: the padded path allocates and then zeroes what the caller can see.
            let p = unsafe { self.alloc_unlocked(layout) };
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

    unsafe fn dealloc_unlocked(&self, ptr: *mut u8, layout: Layout) {
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

    unsafe fn realloc_unlocked(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() > MAX_TRUSTED_ALIGN {
            // `User::ReAlloc` may move the cell, and it promises nothing beyond the
            // heap's own alignment, so an over-aligned block is re-placed by hand.
            // SAFETY: the new layout has the same (valid) alignment and a non-zero size.
            let new = unsafe {
                self.alloc_unlocked(Layout::from_size_align_unchecked(new_size, layout.align()))
            };
            if !new.is_null() {
                // SAFETY: both blocks are live and do not overlap; only the bytes that
                // exist in both are copied.
                unsafe { ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size)) };
                // SAFETY: the old block is still the one this layout describes.
                unsafe { self.dealloc_unlocked(ptr, layout) };
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

// SAFETY: every pointer returned is either a `User::Alloc` cell (alignment 8, observed)
// or a hand-aligned address inside one, sized as the layout asks; `dealloc` gives euser
// back exactly the cell it returned; and all four operations run on one heap, under one
// lock once a second thread exists.
unsafe impl GlobalAlloc for SymbianHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = HeapGuard::enter();
        // SAFETY: the caller's contract, plus the lock this guard holds.
        unsafe { self.alloc_unlocked(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _guard = HeapGuard::enter();
        // SAFETY: as above.
        unsafe { self.alloc_zeroed_unlocked(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _guard = HeapGuard::enter();
        // SAFETY: as above.
        unsafe { self.dealloc_unlocked(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _guard = HeapGuard::enter();
        // SAFETY: as above.
        unsafe { self.realloc_unlocked(ptr, layout, new_size) }
    }
}
