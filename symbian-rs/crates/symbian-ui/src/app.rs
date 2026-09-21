//! `App`: everything an Avkon application has to write, and nothing else.
//!
//! An implementor writes no `unsafe`, names no Symbian type and never sees a leave: the
//! shim owns the four C++ subclasses and this crate owns the boundary. What is left is
//! a struct with state, a `draw` that turns that state into pixels, and a `key` that
//! changes it.
#![forbid(unsafe_code)]

use symbian_core::Result;

use crate::event::{KeyEvent, KeyResponse};
use crate::gc::Gc;
use crate::geom::Rect;
use crate::menu::Menu;
use crate::ui::Ui;

/// The application object the framework owns.
///
/// The order the framework calls these in is fixed (`avkon-rust-spec.md` §3.3): the
/// object is made, then [`App::construct`] is called once with the handles, then any
/// number of [`App::draw`], [`App::key`], [`App::menu`] and [`App::size_changed`],
/// and finally it is dropped.
pub trait App: Sized {
    /// Builds the application's state, before the framework has told it anything.
    ///
    /// It cannot fail and it is handed nothing: everything that needs the view, the
    /// screen size or a file belongs in [`App::construct`]. `fn main` in an application
    /// is exactly one call to this, and is the place to build it some other way.
    fn new() -> Self;

    /// Called once, after the view exists and is on the control stack.
    ///
    /// An error here is reported to the framework, which turns it into a leave **after
    /// this call has returned** — no exception ever crosses a Rust frame.
    fn construct(&mut self, ui: &Ui) -> Result<()> {
        let _ = ui;
        Ok(())
    }

    /// Paints the view. `area` is the whole of it, with its origin at `(0, 0)`.
    ///
    /// This is the one callback the framework makes **outside a trap harness**, so
    /// nothing in it may fail: there is no error channel and nowhere to report one.
    /// Everything [`Gc`] offers is non-leaving by construction, and the stack buffer
    /// behind [`Gc::text`] is why drawing text allocates nothing.
    fn draw(&self, gc: &mut Gc<'_>, area: Rect);

    /// A key the control stack offered this view.
    ///
    /// Return [`KeyResponse::Consumed`] to claim it, or [`KeyResponse::NotConsumed`] so
    /// it travels further — which is what keeps the softkeys and the end key working.
    /// Call [`Ui::redraw`] when the state changed; nothing repaints on its own.
    ///
    /// A key handler has no error channel on purpose: there is nowhere for a failure
    /// here to be reported to.
    fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
        let (_, _) = (event, ui);
        KeyResponse::NotConsumed
    }

    /// The Options menu, declared afresh every time it is opened.
    ///
    /// Nothing about the menu is in `symdev.toml`: the manifest holds what the phone
    /// needs *before* the application runs, and a menu is only ever needed while it
    /// runs. Each line is its label and the code that acts on it, side by side.
    ///
    /// ```ignore
    /// fn menu(&self, m: &mut Menu<Self>) {
    ///     m.item("More bars", |app| app.bars += 1);
    ///     m.item("Reset", |app| app.bars = 3);
    ///     m.exit("Exit");
    /// }
    /// ```
    ///
    /// It is called with `&self` — the framework asks each time the menu opens
    /// (`MEikMenuObserver::DynInitMenuPaneL`), so a menu may depend on the state — and
    /// the action is a plain `fn(&mut Self)`, which is what lets the crate run it
    /// **after** this call has returned rather than aliasing that `&self`. The view is
    /// repainted once the action has returned, so an action never has to ask.
    ///
    /// The left softkey is what opens this, and it exists only when the manifest says
    /// `ui.softkeys = "options-exit"`.
    fn menu(&self, m: &mut Menu<Self>) {
        let _ = m;
    }

    /// The view was resized; `area` is the new one, origin at `(0, 0)`.
    ///
    /// Like [`App::draw`], this is called outside a trap harness and cannot fail.
    fn size_changed(&mut self, area: Rect) {
        let _ = area;
    }
}
