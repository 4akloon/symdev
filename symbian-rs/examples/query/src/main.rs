//! Avkon's modal query dialogs, driven from Rust (step 76 / experiment 93).
//!
//! The application is a form with two fields and no widgets: pressing the selection key
//! asks for a name with [`query::text`], pressing `Up` asks for a number with
//! [`query::number`], and `draw` shows whatever came back. Everything a query needs is
//! in the call — a query is modal, runs its own loop and needs no place on the control
//! stack — so there is no state here beyond the two answers.
//!
//! The acceptance test types into the dialog and reads the answer back **inside** the
//! application, which is the only evidence that matters: a dialog that appears proves
//! `ExecuteLD`, and a dialog whose text reaches `self.name` proves the descriptor the
//! shim handed it was ours.
//!
//! The test drives arrows, digits and the selection key, which is what
//! `docs/research/eka2l1-input.md` says is safe to write a test against — and a data
//! query needs nothing more, because `Return` both opens and confirms one. The softkeys
//! turn out to work here as well, F1 confirming and F2 cancelling: a query dialog's CBA
//! comes from the ROM's own resource, and it is the CBA built from the `.rss` symdev
//! generates that swallows them (experiment 93).
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::ErrorKind;
use symbian_std::test_report::Report;
use symbian_std::ui::prelude::*;
use symbian_std::ui::query;
use symbian_std::write;

/// How many UTF-16 code units the name query accepts. It is here, in the application,
/// because that is the point: `query::text` takes the maximum rather than inventing one.
const NAME_MAX: usize = 32;

/// What the number query starts on, so a screenshot can tell an answer from a default.
const AGE_INITIAL: i32 = 7;

struct Form {
    name: Option<String>,
    age: Option<i32>,
    /// How many queries were cancelled, so a cancel is visible as an outcome rather
    /// than as nothing happening.
    cancelled: u32,
    /// The last `Err` a query returned, as its raw `TInt`.
    failed: Option<i32>,
    /// The two lines `draw` paints, rebuilt whenever an answer changes. `draw` may not
    /// allocate and may not fail, so the formatting happens here instead.
    line: String,
    area: Rect,
}

impl App for Form {
    fn new() -> Self {
        Self {
            name: None,
            age: None,
            cancelled: 0,
            failed: None,
            line: String::new(),
            area: Rect::size(0, 0),
        }
    }

    fn construct(&mut self, ui: &Ui) -> symbian_core::Result<()> {
        self.relabel();
        self.report();
        ui.redraw();
        Ok(())
    }

    fn size_changed(&mut self, area: Rect) {
        self.area = area;
    }

    fn draw(&self, gc: &mut Gc<'_>, area: Rect) {
        gc.set_brush(Rgb::WHITE);
        gc.clear();
        gc.set_pen(Rgb::BLACK);
        gc.text("Select: name   Up: age", Point::new(8, 40));
        gc.text(&self.line, Point::new(8, 80));
        gc.line(Point::new(8, 92), Point::new(area.width - 8, 92));
    }

    fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
        if !event.is_press() {
            return KeyResponse::NotConsumed;
        }
        match event.code {
            // A modal dialog opened from inside `key`. The framework called this
            // through the shim's `OfferKeyEventL`, which is inside a trap harness, and
            // the query's own leaves are trapped one frame further in, in
            // `symrs_query.cpp` — so no leave is ever raised while this frame exists.
            key::SELECT => match query::text("Name?", NAME_MAX) {
                Ok(Some(name)) => self.name = Some(name),
                Ok(None) => self.cancelled += 1,
                Err(e) => self.failed = Some(e.code()),
            },
            key::UP => match query::number("Age?", AGE_INITIAL) {
                Ok(Some(age)) => self.age = Some(age),
                Ok(None) => self.cancelled += 1,
                Err(e) => self.failed = Some(e.code()),
            },
            _ => return KeyResponse::NotConsumed,
        }
        self.relabel();
        self.report();
        ui.redraw();
        KeyResponse::Consumed
    }
}

impl Form {
    /// The one line `draw` paints: every answer this application has, as text.
    ///
    /// `write!` into a `String` cannot fail, and a failure here would be nothing worth
    /// reporting anyway — the line is only ever read by a person looking at a
    /// screenshot — so the result is dropped rather than unwrapped.
    fn relabel(&mut self) {
        self.line.clear();
        let name = self.name.as_deref().unwrap_or("-");
        let _ = write!(self.line, "name={name} age=");
        match self.age {
            Some(age) => {
                let _ = write!(self.line, "{age}");
            }
            None => self.line.push('-'),
        }
        let _ = write!(self.line, " x{}", self.cancelled);
        if let Some(code) = self.failed {
            let _ = write!(self.line, " err{code}");
        }
    }

    /// The result file `symdev test --emulator` reads, rewritten after every query so
    /// that it always describes the run as far as it has got.
    ///
    /// Only what an *undriven* run can honestly assert is a case here: the entry path,
    /// and the guards `query::text` applies before it reaches the framework at all. The
    /// dialogs themselves are not in it, because no case may need a person to press a
    /// key before it can pass — a test that fails until somebody types is a broken
    /// test, not a pending one. The two answers are reported as extra cases once they
    /// exist, so a driven run says six and an undriven one says four, and neither says
    /// anything false. **The screenshots are the test of the dialogs.**
    fn report(&self) {
        let mut report = Report::new("querydemo");
        report.check_detail(
            "the framework reached the Rust construct",
            self.area.width > 0 && self.area.height > 0,
            format_args!("{}x{}", self.area.width, self.area.height),
        );
        // A maximum of zero would be a descriptor nothing can be typed into, and one
        // above the surface's own bound would be an allocation nobody asked for. Both
        // are refused in Rust, before any C++ frame exists.
        report.check_detail(
            "a query with no room for an answer is refused",
            matches!(query::text("", 0), Err(e) if e.kind() == ErrorKind::Argument),
            format_args!("{:?}", query::text("", 0)),
        );
        report.check_detail(
            "a query wanting more than the surface allows is refused",
            matches!(
                query::text("", query::MAX_TEXT_LEN + 1),
                Err(e) if e.kind() == ErrorKind::Argument
            ),
            format_args!("{:?}", query::text("", query::MAX_TEXT_LEN + 1)),
        );
        let long: String = core::iter::repeat_n('x', query::MAX_TEXT_LEN + 1).collect();
        report.check_detail(
            "a prompt longer than the surface allows is refused",
            matches!(query::text(&long, 8), Err(e) if e.kind() == ErrorKind::Overflow),
            format_args!("{:?}", query::text(&long, 8)),
        );
        if let Some(name) = &self.name {
            report.check_detail("a text query came back", true, format_args!("{name:?}"));
        }
        if let Some(age) = self.age {
            report.check_detail("a number query came back", true, format_args!("{age}"));
        }
        if let Some(code) = self.failed {
            report.check_detail("a query returned an error", false, format_args!("{code}"));
        }
        // Writing it is the whole point; there is nowhere to report a failure to write
        // a report to, so the error is dropped deliberately rather than by omission.
        let _ = report.finish();
    }
}

#[symbian_std::main(gui)]
fn main() -> Form {
    Form::new()
}
