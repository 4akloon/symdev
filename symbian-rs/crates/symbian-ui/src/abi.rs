//! The two C structs the Avkon shim and this crate share, and nothing else.
//!
//! They are the Rust half of `symbian-rs/shims/s60/symrs_avkon.h`; the two declarations
//! are kept in step by hand, because they are eight fields each and a bindgen would be
//! a second toolchain on a boundary that changes once a step.
//!
//! Both tables start with a `size` word holding `size_of` as the side that wrote it
//! knows it. The shim checks this crate's table before calling through it, and
//! [`Host::checked`] checks the shim's before this crate does, so a mismatch is a loud
//! failure at startup rather than a read off the end of a table.
use core::ffi::c_void;

/// `TRect` as four `TInt`s.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// `TKeyEvent`, which `w32std.h` line 974 declares as exactly these four words.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawKeyEvent {
    pub code: u32,
    pub scan_code: i32,
    pub modifiers: u32,
    pub repeats: i32,
}

/// "Down": what this crate may ask of the framework.
///
/// Every entry is non-leaving — each one is a pure virtual of `CGraphicsContext` or a
/// non-leaving `CCoeControl`/`CAknAppUi` member — which is what lets `draw` be reached
/// from outside a trap harness at all (`avkon-rust-spec.md` §4.3).
#[repr(C)]
pub struct Host {
    pub size: u32,
    pub clear: unsafe extern "C" fn(*mut c_void),
    pub set_pen: unsafe extern "C" fn(*mut c_void, u32),
    pub set_brush: unsafe extern "C" fn(*mut c_void, u32, i32),
    pub draw_rect: unsafe extern "C" fn(*mut c_void, RawRect),
    pub draw_line: unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32),
    pub draw_text: unsafe extern "C" fn(*mut c_void, *const u16, i32, i32, i32),
    pub redraw: unsafe extern "C" fn(*mut c_void),
    pub exit: unsafe extern "C" fn(*mut c_void),
}

impl Host {
    /// The shim's table, or `None` if it is shorter than this crate's declaration.
    ///
    /// # Safety
    ///
    /// `host` is the pointer the shim passed to `construct`: either null, or a
    /// `&'static SymRsHost` in the shim's `.rodata` whose first word is its own
    /// `sizeof`. Nothing but that first word is read until the size has been checked.
    pub unsafe fn checked<'a>(host: *const Host) -> Option<&'a Host> {
        if host.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees `host` points at a `SymRsHost` whose first
        // field is a `TUint32` size, so reading that one word is in bounds however
        // short the rest of the table is.
        let size = unsafe { core::ptr::read(host.cast::<u32>()) };
        if (size as usize) < size_of::<Host>() {
            return None;
        }
        // SAFETY: the table is at least as long as this declaration, lives in the
        // shim's `.rodata` for the whole process, and is never written.
        Some(unsafe { &*host })
    }
}

/// "Up": what the framework calls on the Rust application object.
///
/// The object is an opaque `*mut c_void` that this crate allocates in `create` and
/// frees in `destroy`; the C++ side never dereferences or deletes it.
#[repr(C)]
pub struct AppVtbl {
    pub size: u32,
    pub create: extern "C" fn() -> *mut c_void,
    pub destroy: extern "C" fn(*mut c_void),
    pub construct: extern "C" fn(*mut c_void, *const Host, *mut c_void, *mut c_void) -> i32,
    pub draw: extern "C" fn(*mut c_void, *mut c_void, RawRect),
    pub offer_key: extern "C" fn(*mut c_void, *const RawKeyEvent, i32) -> i32,
    pub command: extern "C" fn(*mut c_void, i32) -> i32,
    pub size_changed: extern "C" fn(*mut c_void, RawRect),
}
