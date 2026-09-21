//! `Gc`: the drawing operations a `draw` may use, and nothing that can leave.
//!
//! Every call goes through one function pointer in the shim's host table, and every one
//! of those is a pure virtual of `CGraphicsContext` that the SDK does not declare
//! leaving. That is what makes it safe for the framework to call `Draw` outside a trap
//! harness (`avkon-rust-spec.md` §4.3): there is nothing here to trap.
use core::ffi::c_void;

use symbian_core::des::encode_utf16_into;

use crate::abi::Host;
use crate::geom::{Point, Rect, Rgb};

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
    host: &'a Host,
    gc: *mut c_void,
}

impl<'a> Gc<'a> {
    /// # Safety
    ///
    /// `gc` is the `CWindowGc*` the shim passed to `draw` and is valid for the whole
    /// call; `host` is the shim's `.rodata` table, already length-checked.
    pub(crate) const unsafe fn new(host: &'a Host, gc: *mut c_void) -> Self {
        Self { host, gc }
    }

    /// `CWindowGc::Clear()` — the whole clipping region, filled with the brush.
    ///
    /// Set the brush first: the colour a bare `clear` uses is whatever the framework
    /// left behind, which showed up in experiment 76 as an unexplained black band.
    pub fn clear(&mut self) {
        // SAFETY: `clear` is a non-leaving pure virtual of `CGraphicsContext` reached
        // through the shim, and `self.gc` is live for this call by construction.
        unsafe { (self.host.clear)(self.gc) }
    }

    /// `SetPenColor` — the colour of lines and of a rectangle's outline.
    pub fn set_pen(&mut self, colour: Rgb) {
        // SAFETY: as `clear`; `colour` crosses as a plain word.
        unsafe { (self.host.set_pen)(self.gc, colour.bits()) }
    }

    /// `SetBrushStyle(ESolidBrush)` + `SetBrushColor` — what fills a rectangle.
    pub fn set_brush(&mut self, colour: Rgb) {
        // SAFETY: as `clear`.
        unsafe { (self.host.set_brush)(self.gc, colour.bits(), 1) }
    }

    /// `SetBrushStyle(ENullBrush)` — from here on a rectangle is an outline.
    pub fn clear_brush(&mut self) {
        // SAFETY: as `clear`.
        unsafe { (self.host.set_brush)(self.gc, 0, 0) }
    }

    /// `DrawRect` — filled with the brush, outlined with the pen.
    pub fn rect(&mut self, rect: Rect) {
        // SAFETY: as `clear`; `RawRect` is four `TInt`s passed by value, the layout
        // `symrs_avkon.h` declares.
        unsafe { (self.host.draw_rect)(self.gc, rect.raw()) }
    }

    /// `DrawLine`, in the pen colour.
    pub fn line(&mut self, from: Point, to: Point) {
        // SAFETY: as `clear`.
        unsafe { (self.host.draw_line)(self.gc, from.x, from.y, to.x, to.y) }
    }

    /// `DrawText` in the title font, with `at` as the **left end of the baseline** —
    /// the text sits above `at.y`, which is what `DrawText(const TDesC&, const TPoint&)`
    /// means by a position.
    ///
    /// The shim owns the font (`UseFont`/`DiscardFont` never reach Rust), so a `draw`
    /// can never leave one in use. Text beyond [`MAX_TEXT`] code units is truncated.
    pub fn text(&mut self, text: &str, at: Point) {
        let mut units = [0u16; MAX_TEXT];
        let len = match encode_utf16_into(text, &mut units) {
            Ok(len) => len,
            // Longer than the buffer: draw what fits rather than nothing. `char_indices`
            // keeps the cut on a character boundary, and a code unit is at most one
            // `char`, so `MAX_TEXT` characters can never encode short.
            Err(_) => {
                let cut = text
                    .char_indices()
                    .nth(MAX_TEXT / 2)
                    .map_or(text.len(), |(i, _)| i);
                encode_utf16_into(&text[..cut], &mut units).unwrap_or(0)
            }
        };
        if len == 0 {
            return;
        }
        // SAFETY: as `clear`. `units` is a live stack array of `MAX_TEXT` elements and
        // `len <= MAX_TEXT`; the shim wraps the pair in a `TPtrC16` and does not keep
        // it beyond the call.
        unsafe { (self.host.draw_text)(self.gc, units.as_ptr(), len as i32, at.x, at.y) }
    }
}
