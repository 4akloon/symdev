//! The C side of the boundary with `symbian-rs/shims/s60/symrs_avkon.h`: two plain
//! structs and the functions the shim defines.
//!
//! Both directions are plain `extern "C"` symbols resolved by the linker, not tables
//! of function pointers (experiment 104). What Rust asks of the framework ("down") is
//! declared here and defined in the shim; what the framework calls on the application
//! ("up") is `symrs_app_*`, which [`crate::__export_app`] defines in the application
//! crate. A symbol one side expects and the other lacks is a link error — the same
//! loud failure the old tables' `size` words gave at startup, found a step earlier.
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

// "Down": what this crate may ask of the framework.
//
// Every entry is non-leaving — each one is a pure virtual of `CGraphicsContext` or a
// non-leaving `CCoeControl`/`CAknAppUi` member — which is what lets `draw` be reached
// from outside a trap harness at all (`avkon-rust-spec.md` §4.3). The one exception is
// `symrs_menu_add`, documented where it is declared.
unsafe extern "C" {
    pub(crate) fn symrs_gc_clear(gc: *mut c_void, rect: RawRect);
    pub(crate) fn symrs_gc_set_pen(gc: *mut c_void, rgb: u32);
    pub(crate) fn symrs_gc_set_brush(gc: *mut c_void, rgb: u32, solid: i32);
    pub(crate) fn symrs_gc_draw_rect(gc: *mut c_void, rect: RawRect);
    pub(crate) fn symrs_gc_draw_line(gc: *mut c_void, x1: i32, y1: i32, x2: i32, y2: i32);
    pub(crate) fn symrs_gc_draw_text(gc: *mut c_void, text: *const u16, len: i32, x: i32, y: i32);
    pub(crate) fn symrs_view_redraw(view: *mut c_void);
    pub(crate) fn symrs_app_ui_exit(app_ui: *mut c_void);
    /// Adds one line to the Options menu the framework is showing.
    ///
    /// The exception to "every entry is non-leaving": `CEikMenuPane::AddMenuItemL`
    /// leaves on no memory, so the shim traps it and **returns** the error, which the
    /// crate carries back out of `menu` for the shim to raise once the Rust frame has
    /// gone. That is the leave rule of `avkon-rust-spec.md` §4.3, not an exception to
    /// it: nothing throws while a Rust frame is on the stack.
    pub(crate) fn symrs_menu_add(
        pane: *mut c_void,
        text: *const u16,
        len: i32,
        command: i32,
    ) -> i32;
}
