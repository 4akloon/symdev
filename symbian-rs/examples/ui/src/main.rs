//! An Avkon application whose logic is Rust (step 75).
//!
//! The whole application is below: a struct with two numbers, a `draw` that turns them
//! into a bar chart, and a `key` that changes them. There is no `E32Main`, no active
//! scheduler, no `CCoeControl` and no `unsafe` — the C++ subclasses live in the SDK's
//! `shims/s60` and forward every virtual here, and `[ui]` in `symdev.toml` is what
//! asks for them.
//!
//! The acceptance test is a run of PID-bound screenshots with
//! `docs/research/acceptance/emukey.py` between them: three bars, then five after two
//! `Up` presses, then the Options menu on the left softkey (F1), a `Down` and a
//! `Return` to pick an item, and the right softkey (F2) to end the process. The menu
//! itself is the four `m.item` / `m.exit` lines in [`App::menu`] below: nothing about
//! it is in `symdev.toml`, which only says that the left softkey opens one
//! (`softkeys = "options-exit"`).
//!
//! Beside the pixels, `construct` writes the usual result file, so
//! `symdev test --emulator` also has something to say on every rebuild without a
//! person looking at a screenshot. It can only report what is knowable before the
//! first key: that the framework got this far without a leave, in the order the ABI
//! promises, with a view of a plausible size. **The screenshot pair is the test of
//! `draw` and `key`; the report is the test of the entry path.**
#![no_std]

extern crate alloc;

use symbian_std::test_report::Report;
use symbian_std::ui::prelude::*;

/// The heap/startup probe of `docs/research/cpp-parity.md`, pulled in from one shared
/// file so the four examples and their C++ counterparts measure the same two things
/// in the same order. TEMPORARY: this commit exists to take the parity numbers and is
/// reverted immediately after.
#[path = "../../../../docs/research/cpp-parity/probe.rs"]
mod probe;


/// How many bars the chart may show. Six is what fits the E52's client area at the
/// width below without the last one leaving the screen.
const MAX_BARS: u8 = 6;

struct Bars {
    /// The one piece of state the drawing is derived from.
    bars: u8,
    /// How many keys this application has claimed, drawn so a screenshot says
    /// whether a key arrived even when the chart happens to look the same.
    keys: u32,
    /// The same, for menu commands.
    commands: u32,
    /// The view's area, as `size_changed` last reported it. The framework sizes the
    /// view while it is being built, so this is already set when `construct` runs —
    /// which is what lets the report below say how big the client area came out.
    area: Rect,
}

impl App for Bars {
    fn new() -> Self {
        Self {
            bars: 3,
            keys: 0,
            commands: 0,
            area: Rect::size(0, 0),
        }
    }

    fn construct(&mut self, ui: &Ui) -> symbian_core::Result<()> {
        let entry = probe::Probe::now();
        let mut report = Report::new("uidemo");
        // Reaching this callback at all is four facts at once: the shim found
        // `symrs_app_vtbl`, its size word was long enough, `create` returned an object
        // and `BaseConstructL` did not leave.
        report.check("the framework reached the Rust construct", true);
        // The ABI's order (avkon-rust-spec.md §3.3): the view exists and has been
        // sized before `construct`, so `size_changed` has already run.
        report.check_detail(
            "the view was sized before construct",
            self.area.width > 0 && self.area.height > 0,
            format_args!("{}x{}", self.area.width, self.area.height),
        );
        // A redraw may be asked for from anywhere but `draw`; this is the first one.
        ui.redraw();
        report.check("a redraw can be asked for from construct", true);
        let end = probe::Probe::now();
        report.check_detail(
            "probe:heap at entry",
            true,
            format_args!("cells {} bytes {}", entry.cells, entry.bytes),
        );
        report.check_detail(
            "probe:heap at end",
            true,
            format_args!("cells {} bytes {}", end.cells, end.bytes),
        );
        report.check_detail(
            "probe:nanoticks entry to end",
            true,
            format_args!("{}", end.ticks_since(entry)),
        );
        report.check_detail(
            "probe:UserHal::TickPeriod",
            true,
            format_args!("{:?} us", symbian_core::time::SystemTicks::period_micros()),
        );
        report.finish()?;
        Ok(())
    }

    fn size_changed(&mut self, area: Rect) {
        self.area = area;
    }

    fn draw(&self, gc: &mut Gc<'_>, area: Rect) {
        // The brush before the clear: a clear paints with whatever brush the
        // framework left behind otherwise.
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
        note.text(" cmd=");
        note.number(self.commands);
        gc.text(note.as_str(), Point::new(left, baseline + 24));
    }

    /// The Options menu, declared here and nowhere else.
    ///
    /// No id, no constant and no number: the label sits next to the code that acts on
    /// it. The action is a non-capturing closure, so it is a plain `fn(&mut Self)`
    /// that the crate runs after this `&self` call has returned — and the repaint
    /// afterwards is the crate's too.
    fn menu(&self, m: &mut Menu<Self>) {
        m.item("More bars", |app| {
            if app.bars < MAX_BARS {
                app.bars += 1;
                app.commands += 1;
            }
        });
        m.item("Fewer bars", |app| {
            if app.bars > 1 {
                app.bars -= 1;
                app.commands += 1;
            }
        });
        m.item("Reset", |app| {
            app.bars = 3;
            app.commands += 1;
        });
        // The same door the right softkey uses: `EEikCmdExit`, which the shim acts on
        // itself, so no Rust frame is on the stack while the framework tears down.
        m.exit("Exit");
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
