//! The eight `extern "C"` thunks the shim calls, and the table that names them.
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

use crate::abi::{AppVtbl, Host, RawKeyEvent, RawRect};
use crate::app::App;
use crate::command::Command;
use crate::event::{KeyEvent, KeyResponse};
use crate::gc::Gc;
use crate::geom::Rect;
use crate::ui::Ui;

/// What the opaque `void*` really points at: the application and the handles it was
/// given. Boxed once in `create` and dropped once in `destroy`.
struct State<A> {
    app: A,
    ui: Option<Ui>,
}

/// Hands the framework a freshly built application. This is what the `create` thunk
/// `#[symbian_std::main(gui)]` writes calls with the value the application's `fn main`
/// returned.
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

extern "C" fn destroy<A: App>(app: *mut c_void) {
    if app.is_null() {
        return;
    }
    // SAFETY: the shim calls `destroy` exactly once, from the app UI destructor, with
    // the pointer `create` returned; the view that borrowed it is already deleted.
    drop(unsafe { Box::from_raw(app.cast::<State<A>>()) });
}

extern "C" fn construct<A: App>(
    app: *mut c_void,
    host: *const Host,
    view: *mut c_void,
    app_ui: *mut c_void,
) -> i32 {
    // SAFETY: the shim's `ConstructL` passes the pointer `create` returned.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return ErrorKind::Argument.code();
    };
    // SAFETY: `host` is the shim's `.rodata` table; `checked` reads only its size word
    // until that word says the rest is there. A table shorter than this crate's
    // declaration is a shim older than the SDK, and `KErrNotSupported` is what the
    // framework then reports — loudly, through `User::LeaveIfError`.
    let Some(host) = (unsafe { Host::checked(host) }) else {
        return ErrorKind::NotSupported.code();
    };
    // SAFETY: `view` and `app_ui` are the `CShimView*` and `CShimAppUi*` that own this
    // object; both outlive it, because `destroy` runs from the app UI's destructor.
    let ui = state.ui.insert(unsafe { Ui::new(host, view, app_ui) });
    match state.app.construct(ui) {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

extern "C" fn draw<A: App>(app: *mut c_void, gc: *mut c_void, area: RawRect) {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return;
    };
    // `construct` always runs first (the shim's `ConstructL` is what creates the view
    // that can be asked to paint), so `ui` is set. Nothing to report if it is not.
    let Some(ui) = state.ui.as_ref() else {
        return;
    };
    // SAFETY: `gc` is the `CWindowGc&` the framework handed `Draw` and is valid for
    // exactly this call, which is the lifetime `Gc` carries.
    let area = Rect::from_raw(area);
    let mut gc = unsafe { Gc::new(ui.host(), gc, area) };
    state.app.draw(&mut gc, area);
}

extern "C" fn offer_key<A: App>(app: *mut c_void, event: *const RawKeyEvent, kind: i32) -> i32 {
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

extern "C" fn command<A: App>(app: *mut c_void, command: i32) -> i32 {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return ErrorKind::Argument.code();
    };
    let Some(ui) = state.ui.as_ref() else {
        return 0;
    };
    match state.app.command(Command::from_raw(command), ui) {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

extern "C" fn size_changed<A: App>(app: *mut c_void, area: RawRect) {
    // SAFETY: as `construct`.
    let Some(state) = (unsafe { state::<A>(app) }) else {
        return;
    };
    state.app.size_changed(Rect::from_raw(area));
}

impl AppVtbl {
    /// The table for one application type, with the `create` the entry attribute wrote.
    ///
    /// `create` is a parameter rather than `A::new` so that an application's `fn main`
    /// stays the place its object is built: the attribute writes a thunk that calls
    /// `main()` and passes it here.
    pub const fn of<A: App>(create: extern "C" fn() -> *mut c_void) -> Self {
        Self {
            size: size_of::<Self>() as u32,
            create,
            destroy: destroy::<A>,
            construct: construct::<A>,
            draw: draw::<A>,
            offer_key: offer_key::<A>,
            command: command::<A>,
            size_changed: size_changed::<A>,
        }
    }
}
