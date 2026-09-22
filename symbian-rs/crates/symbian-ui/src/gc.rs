//! `Gc`: the drawing operations a `draw` may use, and nothing that can leave.
//!
//! Every call is one `symrs_gc_*` function in the shim, and every one of those is a pure virtual of `CGraphicsContext` that the SDK does not declare
//! leaving. That is what makes it safe for the framework to call `Draw` outside a trap
//! harness (`avkon-rust-spec.md` §4.3): there is nothing here to trap.
use core::ffi::c_void;
use core::marker::PhantomData;

use crate::abi::{
    symrs_gc_clear, symrs_gc_draw_line, symrs_gc_draw_rect, symrs_gc_draw_text, symrs_gc_set_brush,
    symrs_gc_set_pen,
};
use crate::geom::{Point, Rect, Rgb};
use crate::utf16::encode_cut;

/// The greatest number of UTF-16 code units one [`Gc::text`] call draws.
///
/// The buffer is on the stack, because a `draw` that allocates and runs out of memory
/// has no legal way to report it: `Draw` is `const`, is called outside a trap harness,
/// and returns nothing. Text longer than this is drawn truncated rather than dropped.
pub const MAX_TEXT: usize = 128;

/// A borrowed `CWindowGc`, valid only for the `draw` call it was handed to.
///
/// The lifetime is what keeps it that way: a `Gc` cannot be stored in the application
/// struct, because the struct outlives the call.
pub struct Gc<'a> {
    gc: *mut c_void,
    /// The view's own area, so that [`Gc::clear`] can mean "all of it".
    area: Rect,
    /// The `draw` call this context belongs to.
    call: PhantomData<&'a mut c_void>,
}

impl<'a> Gc<'a> {
    /// # Safety
    ///
    /// `gc` is the `CWindowGc*` the shim passed to `draw` and is valid for the whole
    /// call.
    pub(crate) const unsafe fn new(gc: *mut c_void, area: Rect) -> Self {
        Self {
            gc,
            area,
            call: PhantomData,
        }
    }

    /// Fills the whole view with the brush colour, so set the brush first: what a
    /// clear paints otherwise is whatever brush the framework left behind.
    ///
    /// It is `CGraphicsContext::Clear(const TRect&)` over the view's area, and **not**
    /// the no-argument `Clear()`. Observed in EKA2L1 with a probe stripe at the top of
    /// the control: the no-argument form leaves the top ~40 pixels of a window-owning
    /// control unpainted — the black band experiment 76 recorded and could not isolate
    /// — while the rect form covers them.
    pub fn clear(&mut self) {
        // SAFETY: `clear` is a non-leaving pure virtual of `CGraphicsContext` reached
        // through the shim, and `self.gc` is live for this call by construction. The
        // rect is a `TRect` on this frame, which the shim reads for the call only.
        unsafe { symrs_gc_clear(self.gc, &self.area.raw()) }
    }

    /// `SetPenColor` — the colour of lines and of a rectangle's outline.
    pub fn set_pen(&mut self, colour: Rgb) {
        // SAFETY: as `clear`; `colour` crosses as a plain word.
        unsafe { symrs_gc_set_pen(self.gc, colour.bits()) }
    }

    /// `SetBrushStyle(ESolidBrush)` + `SetBrushColor` — what fills a rectangle.
    pub fn set_brush(&mut self, colour: Rgb) {
        // SAFETY: as `clear`.
        unsafe { symrs_gc_set_brush(self.gc, colour.bits(), 1) }
    }

    /// `SetBrushStyle(ENullBrush)` — from here on a rectangle is an outline.
    pub fn clear_brush(&mut self) {
        // SAFETY: as `clear`.
        unsafe { symrs_gc_set_brush(self.gc, 0, 0) }
    }

    /// `DrawRect` — filled with the brush, outlined with the pen.
    pub fn rect(&mut self, rect: Rect) {
        // SAFETY: as `clear`.
        unsafe { symrs_gc_draw_rect(self.gc, &rect.raw()) }
    }

    /// `DrawLine`, in the pen colour.
    pub fn line(&mut self, from: Point, to: Point) {
        // SAFETY: as `clear`.
        unsafe { symrs_gc_draw_line(self.gc, from.x, from.y, to.x, to.y) }
    }

    /// `DrawText` in the title font, with `at` as the **left end of the baseline** —
    /// the text sits above `at.y`, which is what `DrawText(const TDesC&, const TPoint&)`
    /// means by a position.
    ///
    /// The shim owns the font (`UseFont`/`DiscardFont` never reach Rust), so a `draw`
    /// can never leave one in use. Text beyond [`MAX_TEXT`] code units is truncated.
    pub fn text(&mut self, text: &str, at: Point) {
        let mut units = [0u16; MAX_TEXT];
        let len = encode_cut(text, &mut units);
        if len == 0 {
            return;
        }
        // SAFETY: as `clear`. `units` is a live stack array of `MAX_TEXT` elements and
        // `len <= MAX_TEXT`; the shim wraps the pair in a `TPtrC16` and does not keep
        // it beyond the call.
        unsafe { symrs_gc_draw_text(self.gc, units.as_ptr(), len as i32, at.x, at.y) }
    }
}
