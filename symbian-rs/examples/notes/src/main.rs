//! The four Avkon notes, one per arrow key (step for experiment 92).
//!
//! `User::InfoPrint` is what every example used to report with; it is a server round
//! trip that draws a system dialog of its own and looks nothing like the platform. A
//! note is the real thing: `symbian_std::ui::note::info("Saved")` and three siblings.
//!
//! The application draws which note it last asked for and how long the call took, so a
//! screenshot says both what appeared and whether the call blocked. Timing is
//! `std::time::Instant`, whose tick is 15 625 µs here (experiment 85) — coarse, and
//! coarse is enough: a note that waits is dismissed by a person or by a 1.5–3 s
//! timeout, which is 96–192 ticks, and one that does not wait is 0.
//!
//! Acceptance: `emukey.py keys <pid> Left`, screenshot — an information note over the
//! view — against the same frame with no note on it.
#![no_std]

extern crate alloc;

use symbian_std::test_report::{Report, detail};
use symbian_std::time::Instant;
use symbian_std::ui::note;
use symbian_std::ui::prelude::*;

struct Notes {
    /// The last note asked for, as the label drawn on screen.
    last: &'static str,
    /// How long that call took, in microseconds, measured around the call itself.
    micros: u64,
    /// The Symbian error the last call returned, or 0. Drawn so a failure is visible
    /// on the screenshot instead of being silently nothing.
    err: i32,
    /// How many notes have been asked for, so a screenshot says a key arrived even
    /// when the note has already timed out.
    shown: u32,
    area: Rect,
}

impl Notes {
    /// Shows one note and records what it cost. The `Result` is kept rather than
    /// thrown away: an application that cannot tell the user something should be able
    /// to notice.
    fn show(&mut self, label: &'static str, f: fn(&str) -> symbian_core::Result<()>, text: &str) {
        let started = Instant::now().ok();
        let outcome = f(text);
        self.micros = match started {
            Some(started) => started.elapsed().as_micros() as u64,
            None => 0,
        };
        self.last = label;
        self.err = match outcome {
            Ok(()) => 0,
            Err(e) => e.code(),
        };
        self.shown += 1;
    }
}

impl App for Notes {
    fn new() -> Self {
        Self {
            last: "none",
            micros: 0,
            err: 0,
            shown: 0,
            area: Rect::size(0, 0),
        }
    }

    fn construct(&mut self, ui: &Ui) -> symbian_core::Result<()> {
        let mut report = Report::new("notes");
        // The question the shim could not answer from a header: does a note need the
        // application environment to exist first, i.e. can one be shown this early?
        // `construct` runs after the shim's `BaseConstructL`, so it should.
        let started = Instant::now().ok();
        let shown = note::info("Notes ready");
        let micros = started.map_or(0, |s| s.elapsed().as_micros() as u64);
        report.check_detail(
            "an information note can be shown from construct",
            shown.is_ok(),
            detail!("{:?}", shown.as_ref().err().map(|e| e.code())),
        );
        // The modality answer, measured rather than assumed. `R_AKN_INFORMATION_NOTE`
        // carries no `EEikDialogFlagWait`, so `ExecuteLD` should return while the note
        // is still on screen: well under one 15 625 µs tick.
        report.check_detail(
            "ExecuteLD returned without waiting for the note to close",
            micros < 500_000,
            detail!("{micros} us"),
        );
        report.check_detail(
            "the view was sized before construct",
            self.area.width > 0 && self.area.height > 0,
            detail!("{}x{}", self.area.width, self.area.height),
        );
        self.last = "info";
        self.micros = micros;
        self.shown = 1;
        ui.redraw();
        report.finish()?;
        Ok(())
    }

    fn size_changed(&mut self, area: Rect) {
        self.area = area;
    }

    fn draw(&self, gc: &mut Gc<'_>, area: Rect) {
        gc.set_brush(Rgb::WHITE);
        gc.clear();
        gc.set_pen(Rgb::BLACK);

        let left = 10;
        let mut y = 28;
        let step = 22;
        for line in [
            "Left   information",
            "Right  confirmation",
            "Up     warning",
            "Down   error",
        ] {
            gc.text(line, Point::new(left, y));
            y += step;
        }

        gc.line(
            Point::new(left, y - 12),
            Point::new(area.width - left, y - 12),
        );
        y += 8;

        let mut line = Text::new();
        line.str("last=");
        line.str(self.last);
        gc.text(line.as_str(), Point::new(left, y));
        y += step;

        let mut line = Text::new();
        line.str("shown=");
        line.number(self.shown as u64);
        line.str(" err=");
        if self.err < 0 {
            line.str("-");
            line.number(self.err.unsigned_abs() as u64);
        } else {
            line.number(self.err as u64);
        }
        gc.text(line.as_str(), Point::new(left, y));
        y += step;

        let mut line = Text::new();
        line.str("us=");
        line.number(self.micros);
        gc.text(line.as_str(), Point::new(left, y));
    }

    fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
        if !event.is_press() {
            return KeyResponse::NotConsumed;
        }
        match event.code {
            key::LEFT => self.show("info", note::info, "Saved"),
            key::RIGHT => self.show("confirm", note::confirm, "Message sent"),
            key::UP => self.show("warn", note::warn, "Battery low"),
            key::DOWN => self.show("error", note::error, "No network"),
            // Everything else travels on, so the softkeys and the end key still work.
            _ => return KeyResponse::NotConsumed,
        }
        ui.redraw();
        KeyResponse::Consumed
    }
}

#[symbian_std::main(gui)]
fn main() -> Notes {
    Notes::new()
}

/// A line of text built on the stack, because `draw` may not allocate and `core::fmt`
/// costs about 2.4 kB (experiment 77) that this example's size figure should not hide.
struct Text {
    buf: [u8; 48],
    len: usize,
}

impl Text {
    const fn new() -> Self {
        Self {
            buf: [0; 48],
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
    fn str(&mut self, s: &str) {
        for byte in s.bytes().filter(u8::is_ascii) {
            self.push(byte);
        }
    }

    fn number(&mut self, mut value: u64) {
        let mut digits = [0u8; 20];
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
