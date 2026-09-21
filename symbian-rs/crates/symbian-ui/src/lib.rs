//! An Avkon application whose logic is Rust (design spec §11 step 75).
//!
//! The Symbian application framework is four C++ classes with virtual methods, so the
//! subclasses live in C++ — `symbian-rs/shims/s60/symrs_avkon.cpp`, compiled into the
//! application by symdev when the manifest has a `[ui]` section — and every virtual is
//! forwarded through one table of function pointers to this crate. What an application
//! writes is a struct and an [`App`] impl:
//!
//! ```ignore
//! #![no_std]
//!
//! use symbian_ui::prelude::*;
//!
//! struct Bars {
//!     bars: u8,
//! }
//!
//! impl App for Bars {
//!     fn new() -> Self {
//!         Self { bars: 3 }
//!     }
//!
//!     fn draw(&self, gc: &mut Gc<'_>, area: Rect) {
//!         gc.set_brush(Rgb::WHITE);
//!         gc.clear();
//!         gc.text("hello", Point::new(8, 24));
//!     }
//!
//!     fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
//!         if event.is_press() && event.code == key::UP {
//!             self.bars += 1;
//!             ui.redraw();
//!             return KeyResponse::Consumed;
//!         }
//!         KeyResponse::NotConsumed
//!     }
//!
//!     fn command(&mut self, command: Command, ui: &Ui) -> symbian_core::Result<()> {
//!         if command.is("more") {
//!             self.bars += 1;
//!             ui.redraw();
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[symbian_std::main(gui)]
//! fn main() -> Bars {
//!     Bars::new()
//! }
//! ```
//!
//! # The Options menu, and where a softkey goes
//!
//! The menu is declared in `symdev.toml`, not here, because it is a compiled resource
//! that has to exist before any Rust runs:
//!
//! ```toml
//! [[ui.menu]]
//! id = "more"
//! label = "More bars"
//! ```
//!
//! and the *word* `more` is what [`Command::named`] matches — symdev and this crate
//! derive the same number from it, so neither the manifest nor the source writes one
//! down. The left softkey opens that menu and the right one ends the application; both
//! are handled below this crate, so neither reaches [`App::key`]. A softkey never
//! does: the button group container sits above the view on the control stack and
//! turns the key into a command.
//!
//! There is no `E32Main` here and no active scheduler: for a GUI application the shim
//! owns the entry point and hands the process to `EikStart::RunApplication`, and CONE
//! creates and runs the `CCoeScheduler` itself (`CCoeEnv` is a `CActive` on it). An
//! application that installed a second `CActiveScheduler` would hang.
//!
//! # What this crate guarantees
//!
//! * **No leave ever reaches Rust.** rustc marks the whole Rust text `cantunwind`, so
//!   a C++ exception crossing it ends the process with no diagnostic at all
//!   (experiment 76). Every host entry point is non-leaving or is trapped inside the
//!   shim, and an application's error becomes a leave only after its frame has
//!   returned. The rule, per virtual, is in `docs/research/avkon-rust-spec.md` §4.3.
//! * **An application writes no `unsafe`.** Every raw pointer the framework hands over
//!   is turned into a borrow in [`vtbl`], which is the only module here that
//!   dereferences one.
//! * **`draw` cannot fail.** It is the one callback the framework makes outside a trap
//!   harness, so it has no error channel and [`Gc`] has nothing in it that can fail.
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod note;

mod abi;
mod app;
mod command;
mod event;
mod gc;
mod geom;
mod list;
pub mod query;
mod ui;
mod vtbl;

pub use abi::AppVtbl;
pub use app::App;
pub use command::Command;
pub use event::{EventCode, KeyEvent, KeyResponse, key, scan};
pub use gc::{Gc, MAX_TEXT};
pub use geom::{Point, Rect, Rgb};
pub use list::{List, MAX_ITEM_TEXT};
pub use ui::Ui;
pub use vtbl::start;

/// Everything an [`App`] implementation names, in one `use`.
pub mod prelude {
    pub use crate::app::App;
    pub use crate::command::Command;
    pub use crate::event::{EventCode, KeyEvent, KeyResponse, key, scan};
    pub use crate::gc::Gc;
    pub use crate::geom::{Point, Rect, Rgb};
    pub use crate::list::List;
    pub use crate::ui::Ui;
}
