//! The bodies of the eight `symrs_app_*` functions the shim calls, and the macro that
//! exports them from the application crate.
//!
//! They are generic over the application type, and that costs nothing: an image holds
//! exactly one `App`, so each body is instantiated once, and [`crate::__export_app`]
//! gives each instance its C name with a one-line `extern "C"` wrapper that inlines
//! away. The shim calls them directly — no table, no function pointer (experiment 104).
//!
//! This is the only module in the crate that dereferences what C++ hands it, and the
//! only place an application's `&mut` is taken. The rules it keeps, in the terms of
//! `avkon-rust-spec.md` §4.3:
//!
//! * nothing here leaves, panics or unwinds — a Rust panic is `abort` and a C++
//!   exception reaching this frame would end the process with no diagnostic;
//! * a failure becomes a returned `TInt`, and the shim turns it into a leave **after**
//!   this frame has returned;
//! * `draw` and `size_changed` have no error channel at all, because the framework
//!   calls them outside a trap harness.
use core::ffi::c_void;

use alloc::boxed::Box;
use symbian_core::ErrorKind;

use crate::abi::{RawKeyEvent, RawRect};
use crate::app::App;
use crate::event::{KeyEvent, KeyResponse};
use crate::gc::Gc;
use crate::geom::Rect;
use crate::menu::{Menu, index_of};
use crate::ui::Ui;

/// What the opaque `void*` really points at: the application and the handles it was
/// given. Boxed once in `create` and dropped once in `destroy`.
struct State<A> {
    app: A,
    ui: Option<Ui>,
}

/// Hands the framework a freshly built application. This is what `symrs_app_create`
/// calls with the value the application's `fn main` returned.
pub fn start<A: App>(app: A) -> *mut c_void {
    Box::into_raw(Box::new(State { app, ui: None })).cast()
}

/// # Safety
///
/// `app` is a pointer `start` returned for the same `A` and `destroy` has not yet been
/// called on it. The framework calls every one of these on its single thread, so no
/// two of these borrows are ever live at once.
unsafe fn state<'a, A: App>(app: *mut c_void) -> Option<&'a mut State<A>> {
    if app.is_null() {
        return None;
    }
    // SAFETY: by the contract above the pointer came from `Box::into_raw` of a
    // `State<A>` that is still alive, and the framework is single-threaded.
    Some(unsafe { &mut *app.cast::<State<A>>() })
}

/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn destroy<A: App>(app: *mut c_void) {
    if app.is_null() {
        return;
    }
    // SAFETY: the shim calls `destroy` exactly once, from the app UI destructor, with
    // the pointer `create` returned; the view that borrowed it is already deleted.
    drop(unsafe { Box::from_raw(app.cast::<State<A>>()) });
}

/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn construct<A: App>(app: *mut c_void, view: *mut c_void, app_ui: *mut c_void) -> i32 {
    // SAFETY: the shim's `ConstructL` passes the pointer `create` returned.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return ErrorKind::Argument.code();
    };
    // SAFETY: `view` and `app_ui` are the `CShimView*` and `CShimAppUi*` that own this
    // object; both outlive it, because `destroy` runs from the app UI's destructor.
    let ui = state.ui.insert(unsafe { Ui::new(view, app_ui) });
    match state.app.construct(ui) {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn draw<A: App>(app: *mut c_void, gc: *mut c_void, area: RawRect) {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return;
    };
    // `construct` always runs first (the shim's `ConstructL` is what creates the view
    // that can be asked to paint), so `ui` is set. Nothing to report if it is not.
    if state.ui.is_none() {
        return;
    }
    // SAFETY: `gc` is the `CWindowGc&` the framework handed `Draw` and is valid for
    // exactly this call, which is the lifetime `Gc` carries.
    let area = Rect::from_raw(area);
    let mut gc = unsafe { Gc::new(gc, area) };
    state.app.draw(&mut gc, area);
}

/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn offer_key<A: App>(app: *mut c_void, event: *const RawKeyEvent, kind: i32) -> i32 {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return KeyResponse::NotConsumed as i32;
    };
    if event.is_null() {
        return KeyResponse::NotConsumed as i32;
    }
    // SAFETY: the shim fills a `SymRsKeyEvent` on its own stack from the `TKeyEvent`
    // the framework gave it and passes it by reference for the duration of the call.
    let raw = unsafe { &*event };
    let Some(ui) = state.ui.as_ref() else {
        return KeyResponse::NotConsumed as i32;
    };
    state.app.key(KeyEvent::from_raw(raw, kind), ui) as i32
}

/// `DynInitMenuPaneL`: the framework is about to show the Options menu.
///
/// The application declares it afresh each time, which is what `AddMenuItemL`'s own
/// documentation calls adding an item "dynamically". A failure is returned, and the
/// shim raises it once this frame has gone.
/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn menu<A: App>(app: *mut c_void, pane: *mut c_void) -> i32 {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return ErrorKind::Argument.code();
    };
    if pane.is_null() {
        return ErrorKind::Argument.code();
    }
    if state.ui.is_none() {
        return 0;
    }
    // SAFETY: `pane` is the `CEikMenuPane*` the framework handed `DynInitMenuPaneL`
    // and is valid for exactly this call, which is the lifetime `Menu` carries.
    let mut items = unsafe { Menu::fill(pane) };
    state.app.menu(&mut items);
    items.error()
}

/// `HandleCommandL`, for everything the shim did not act on itself.
///
/// The application never sees the number. It is the position of a line of the menu the
/// application last declared, so the menu is declared once more — with `&self`, which
/// is all [`App::menu`] ever gets — the action at that position is **copied out** as a
/// `fn` pointer, and only then, with no borrow of the application left alive, is it
/// called with `&mut`. That is why an action can take `&mut A` at all.
///
/// The repaint afterwards is the crate's, not the action's: an action is handed the
/// application and nothing else, so it has no way to ask for one — and no way to
/// re-enter the framework, which is what makes a nested callback impossible while
/// that `&mut` is live.
/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn command<A: App>(app: *mut c_void, command: i32) -> i32 {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return ErrorKind::Argument.code();
    };
    let Some(index) = index_of(command) else {
        return 0;
    };
    let mut lookup = Menu::find(index);
    state.app.menu(&mut lookup);
    let Some(action) = lookup.action() else {
        return 0;
    };
    action(&mut state.app);
    if let Some(ui) = state.ui.as_ref() {
        ui.redraw();
    }
    0
}

/// # Safety
///
/// `app` came from [`start`] for this `A` and is alive, and every other pointer is
/// what `symrs_avkon.h` documents for this call.
#[doc(hidden)]
pub unsafe fn size_changed<A: App>(app: *mut c_void, area: RawRect) {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return;
    };
    state.app.size_changed(Rect::from_raw(area));
}

/// Exports the eight `symrs_app_*` functions `symbian-rs/shims/s60/symrs_avkon.h`
/// declares, for the application type `$app` built by `$main`.
///
/// `#[symbian_std::main(gui)]` writes the one call; nothing else should. `create` is
/// `$main` rather than `App::new` so that an application's `fn main` stays the place
/// its object is built.
#[doc(hidden)]
#[macro_export]
macro_rules! __export_app {
    ($app:ty, $main:path) => {
        const _: () = {
            use ::core::ffi::c_void;
            use $crate::__abi::{RawKeyEvent, RawRect};
            use $crate::__glue as glue;

            #[unsafe(export_name = "symrs_app_create")]
            extern "C" fn create() -> *mut c_void {
                $crate::start::<$app>($main())
            }
            // SAFETY (all seven): the shim passes the pointer `create` returned and the
            // arguments `symrs_avkon.h` documents, which is each body's contract.
            #[unsafe(export_name = "symrs_app_destroy")]
            extern "C" fn destroy(app: *mut c_void) {
                unsafe { glue::destroy::<$app>(app) }
            }
            #[unsafe(export_name = "symrs_app_construct")]
            extern "C" fn construct(app: *mut c_void, view: *mut c_void, ui: *mut c_void) -> i32 {
                unsafe { glue::construct::<$app>(app, view, ui) }
            }
            #[unsafe(export_name = "symrs_app_draw")]
            extern "C" fn draw(app: *mut c_void, gc: *mut c_void, area: RawRect) {
                unsafe { glue::draw::<$app>(app, gc, area) }
            }
            #[unsafe(export_name = "symrs_app_offer_key")]
            extern "C" fn offer_key(app: *mut c_void, event: *const RawKeyEvent, kind: i32) -> i32 {
                unsafe { glue::offer_key::<$app>(app, event, kind) }
            }
            #[unsafe(export_name = "symrs_app_command")]
            extern "C" fn command(app: *mut c_void, command: i32) -> i32 {
                unsafe { glue::command::<$app>(app, command) }
            }
            #[unsafe(export_name = "symrs_app_size_changed")]
            extern "C" fn size_changed(app: *mut c_void, area: RawRect) {
                unsafe { glue::size_changed::<$app>(app, area) }
            }
            #[unsafe(export_name = "symrs_app_menu")]
            extern "C" fn menu(app: *mut c_void, pane: *mut c_void) -> i32 {
                unsafe { glue::menu::<$app>(app, pane) }
            }
        };
    };
}
