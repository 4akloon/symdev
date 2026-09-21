//! An Avkon application whose logic is Rust (step 75).
//!
//! The whole application is below: a struct with two numbers, a `draw` that turns them
//! into a bar chart, and a `key` that changes them. There is no `E32Main`, no active
//! scheduler, no `CCoeControl` and no `unsafe` — the C++ subclasses live in the SDK's
//! `shims/s60` and forward every virtual here, and `[ui]` in `symdev.toml` is what
//! asks for them.
//!
//! The acceptance test is a pair of PID-bound screenshots with
//! `docs/research/acceptance/emukey.py` between them: three bars before, five after
//! two `Up` presses. It is written against the arrows and the selection key and never
//! against a softkey, because F1/F2 reach the guest and still do nothing in an
//! application built here (`docs/research/eka2l1-input.md`).
#![no_std]

use symbian_std::ui::prelude::*;

/// How many bars the chart may show. Six is what fits the E52's client area at the
/// width below without the last one leaving the screen.
const MAX_BARS: u8 = 6;

struct Bars {
    /// The one piece of state the drawing is derived from.
    bars: u8,
    /// How many keys this application has claimed, drawn so a screenshot says
    /// whether a key arrived even when the chart happens to look the same.
    keys: u32,
}

impl App for Bars {
    fn new() -> Self {
        Self { bars: 3, keys: 0 }
    }

    fn draw(&self, gc: &mut Gc<'_>, area: Rect) {
        // The brush before the clear: `Clear()` fills with whatever brush the
        // framework left behind, which is the unexplained black band of experiment 76.
        gc.set_brush(Rgb::WHITE);
        gc.clear();

        let baseline = area.height - 40;
        let width = 20;
        let gap = 10;
        let left = 12;
        gc.set_pen(Rgb::BLACK);
        for i in 0..self.bars as i32 {
            let height = 20 + i * 18;
            gc.set_brush(Rgb::new(32, 111, 235));
            gc.rect(Rect::new(
                left + i * (width + gap),
                baseline - height,
                width,
                height,
            ));
        }
        gc.line(
            Point::new(left, baseline),
            Point::new(area.width - left, baseline),
        );

        let mut note = Note::new();
        note.text("bars=");
        note.number(self.bars as u32);
        note.text(" keys=");
        note.number(self.keys);
        gc.text(note.as_str(), Point::new(left, baseline + 24));
    }

    fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
        // Only the event that carries a character code; the up and down events for
        // the same press have `code == 0`.
        if !event.is_press() {
            return KeyResponse::NotConsumed;
        }
        match event.code {
            key::UP if self.bars < MAX_BARS => self.bars += 1,
            key::DOWN if self.bars > 1 => self.bars -= 1,
            key::SELECT => self.bars = 3,
            // Everything else travels on, so the softkeys and the end key still work.
            _ => return KeyResponse::NotConsumed,
        }
        self.keys += 1;
        // Nothing repaints on its own, and this schedules the repaint rather than
        // re-entering `draw` while this frame still holds the application borrowed.
        ui.redraw();
        KeyResponse::Consumed
    }
}

#[symbian_std::main(gui)]
fn main() -> Bars {
    Bars::new()
}

/// A line of text built on the stack.
///
/// `Gc::text` takes a `&str` and `draw` may not allocate, so the two numbers are
/// rendered by hand rather than with `format!`. `core::fmt` would work and cost about
/// 2.4 kB (experiment 77); this example is also the size measurement for the step, so
/// it stays out of it.
struct Note {
    buf: [u8; 32],
    len: usize,
}

impl Note {
    const fn new() -> Self {
        Self {
            buf: [0; 32],
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) {
        if self.len < self.buf.len() {
            self.buf[self.len] = byte;
            self.len += 1;
        }
    }

    /// ASCII only, which every caller here is; anything else is dropped rather than
    /// cutting a UTF-8 sequence in half.
    fn text(&mut self, s: &str) {
        for byte in s.bytes().filter(u8::is_ascii) {
            self.push(byte);
        }
    }

    fn number(&mut self, mut value: u32) {
        let mut digits = [0u8; 10];
        let mut n = 0;
        loop {
            digits[n] = b'0' + (value % 10) as u8;
            value /= 10;
            n += 1;
            if value == 0 {
                break;
            }
        }
        while n > 0 {
            n -= 1;
            self.push(digits[n]);
        }
    }

    fn as_str(&self) -> &str {
        // Every byte came from `push`, which only ever takes an ASCII one.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}
