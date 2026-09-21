//! `App`: everything an Avkon application has to write, and nothing else.
//!
//! An implementor writes no `unsafe`, names no Symbian type and never sees a leave: the
//! shim owns the four C++ subclasses and this crate owns the boundary. What is left is
//! a struct with state, a `draw` that turns that state into pixels, and a `key` that
//! changes it.
#![forbid(unsafe_code)]

use symbian_core::Result;

use crate::command::Command;
use crate::event::{KeyEvent, KeyResponse};
use crate::gc::Gc;
use crate::geom::Rect;
use crate::ui::Ui;

/// The application object the framework owns.
///
/// The order the framework calls these in is fixed (`avkon-rust-spec.md` §3.3): the
/// object is made, then [`App::construct`] is called once with the handles, then any
/// number of [`App::draw`], [`App::key`], [`App::command`] and [`App::size_changed`],
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

    /// An item of the Options menu, named by the word `[[ui.menu]] id` gave it.
    ///
    /// `symdev.toml` declares the menu, because it is a compiled resource that has to
    /// exist before any Rust runs; this is where it is acted on. Match with
    /// [`Command::named`] or [`Command::is`] and the word is the only thing written
    /// twice.
    ///
    /// Two commands never arrive: the right softkey's [`Command::EXIT`], which the
    /// shim acts on itself, and [`Command::OPTIONS`], which the framework consumes to
    /// open the menu. An error here is turned into a leave **after** this call has
    /// returned.
    fn command(&mut self, command: Command, ui: &Ui) -> Result<()> {
        let (_, _) = (command, ui);
        Ok(())
    }

    /// The view was resized; `area` is the new one, origin at `(0, 0)`.
    ///
    /// Like [`App::draw`], this is called outside a trap harness and cannot fail.
    fn size_changed(&mut self, area: Rect) {
        let _ = area;
    }
}
