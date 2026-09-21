//! An S60 list box driven from Rust, and nothing else (step 75, the list slice).
//!
//! The whole application is below: twelve rows, and a callback that rewrites the first
//! one with the index of whatever was chosen. There is no `E32Main`, no `CCoeControl`,
//! no Symbian type and no `unsafe` — `symrs_list.cpp` owns the `CAknSingleStyleListBox`
//! and `[ui]` in `symdev.toml` is what asks for the shim at all.
//!
//! # Why the status row, and not `draw`
//!
//! The list is a full-screen, window-owning control sitting **above** the application's
//! view on the control stack, which is what makes it consume the arrows before the view
//! sees them. The price is that it covers the view completely: `App::draw` still runs
//! and still paints, and none of it is visible. So the only honest place for this
//! application to show what it was told is row 0 of the list itself — which is also a
//! better proof, because it can only be written by a Rust callback that was handed the
//! right index.
//!
//! # The acceptance test
//!
//! PID-bound screenshots either side of `docs/research/acceptance/emukey.py`:
//!
//! 1. shot — twelve rows, `picked: -` at the top, the highlight on row 0;
//! 2. `emukey.py keys <pid> Down Down Down` — the highlight is three rows lower and the
//!    application has seen no key at all, because the list consumed every one;
//! 3. `emukey.py keys <pid> Return` — row 0 now reads `picked: 3`.
//!
//! Arrows and the selection key only. F1/F2 reach the guest and still do nothing in an
//! application built here (`docs/research/eka2l1-input.md`), so no softkey is in it.
#![no_std]

extern crate alloc;

use symbian_std::test_report::Report;
use symbian_std::ui::prelude::*;

/// The rows below the status row. Twelve of them, so that the list overflows the E52's
/// 240x245 client area and the scroll bar has something to do.
const ITEMS: [&str; 12] = [
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett",
    "Kilo", "Lima",
];

/// What row 0 says before anything has been chosen.
const NOTHING_PICKED: &str = "picked: -";

struct ListDemo {
    /// The control. It is on the control stack for as long as this field is, so the
    /// application struct is where it has to live.
    list: Option<List>,
    /// The view's area, as `size_changed` last reported it — the one thing worth
    /// reporting that is knowable before the first key.
    area: Rect,
}

/// The full set of rows: the status row, then [`ITEMS`].
fn rows(status: &str) -> [&str; 13] {
    let mut all = [status; 13];
    all[1..].copy_from_slice(&ITEMS);
    all
}

impl App for ListDemo {
    fn new() -> Self {
        Self {
            list: None,
            area: Rect::size(0, 0),
        }
    }

    fn construct(&mut self, ui: &Ui) -> symbian_core::Result<()> {
        let mut list = List::new(ui, &rows(NOTHING_PICKED))?;
        // The callback cannot reach this struct — the framework called the list, not
        // the view — so what it is given is the rows. `Label` is built on the stack,
        // which is why nothing here allocates a string.
        list.on_select(|index, rows| {
            let mut label = Label::new();
            label.text("picked: ");
            label.number(index as u32);
            // A failed rewrite has nowhere to be reported from a callback, and leaving
            // the rows as they were is the truthful outcome.
            let _ = rows.set_items(&self::rows(label.as_str()));
        });

        let mut report = Report::new("listdemo");
        report.check("the framework reached the Rust construct", true);
        report.check_detail(
            "the view was sized before construct",
            self.area.width > 0 && self.area.height > 0,
            format_args!("{}x{}", self.area.width, self.area.height),
        );
        report.check_detail(
            "the list holds every row it was given",
            list.len() == ITEMS.len() + 1,
            format_args!("{} rows", list.len()),
        );
        report.check_detail(
            "the first row is selected to begin with",
            list.selected() == 0,
            format_args!("index {}", list.selected()),
        );
        // Moving the highlight from Rust works, and moving it back leaves the list
        // where the screenshot expects it.
        list.set_selected(ITEMS.len())?;
        let moved = list.selected() == ITEMS.len();
        list.set_selected(0)?;
        report.check_detail(
            "the highlight can be moved from Rust",
            moved && list.selected() == 0,
            format_args!("index {}", list.selected()),
        );
        report.check(
            "an index past the end is refused",
            list.set_selected(ITEMS.len() + 1).is_err(),
        );
        self.list = Some(list);
        report.finish()?;
        Ok(())
    }

    fn size_changed(&mut self, area: Rect) {
        self.area = area;
    }

    /// The list covers all of this. It is still painted, because a view that leaves its
    /// window undefined is a view that shows whatever was there before if the list is
    /// ever made smaller than the client area.
    fn draw(&self, gc: &mut Gc<'_>, _area: Rect) {
        gc.set_brush(Rgb::WHITE);
        gc.clear();
    }

    /// Nothing reaches here while the list has focus: it is above this view on the
    /// control stack and claims the arrows and the selection key itself. The method is
    /// left in to say so, and to keep every other key travelling on to the framework.
    fn key(&mut self, _event: KeyEvent, _ui: &Ui) -> KeyResponse {
        KeyResponse::NotConsumed
    }
}

#[symbian_std::main(gui)]
fn main() -> ListDemo {
    ListDemo::new()
}

/// A short line of text built on the stack.
///
/// The callback runs inside the list's own `OfferKeyEventL` and a `format!` would both
/// allocate and pull `core::fmt` into an example whose size is being measured.
struct Label {
    buf: [u8; 24],
    len: usize,
}

impl Label {
    const fn new() -> Self {
        Self {
            buf: [0; 24],
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
