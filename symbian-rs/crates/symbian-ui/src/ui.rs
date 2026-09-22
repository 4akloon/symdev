//! `Ui`: what the application may ask of the framework outside a `draw`.
//!
//! It holds the two opaque handles the shim passed to `construct` — the view and the
//! app UI. Both live exactly as long as the application object, so a `&Ui` is handed
//! to every callback that may act on them.
use core::ffi::c_void;

use crate::abi::{symrs_app_ui_exit, symrs_view_redraw};

pub struct Ui {
    view: *mut c_void,
    app_ui: *mut c_void,
}

impl Ui {
    /// # Safety
    ///
    /// The two pointers are the ones the shim passed to `construct`: the `CShimView*`
    /// and `CShimAppUi*` that own this application object and are destroyed only after
    /// `destroy` has been called.
    pub(crate) const unsafe fn new(view: *mut c_void, app_ui: *mut c_void) -> Self {
        Self { view, app_ui }
    }

    /// Asks for the view to be painted again.
    ///
    /// This **schedules** a redraw (`CCoeControl::DrawDeferred`); it does not call
    /// `draw` from inside this call. That is deliberate: a synchronous repaint would
    /// re-enter the application object while a `key` still holds it borrowed.
    pub fn redraw(&self) {
        // SAFETY: `DrawDeferred` is a non-leaving `CCoeControl` member reached through
        // the shim, and `self.view` is owned by the app UI that owns this object.
        unsafe { symrs_view_redraw(self.view) }
    }

    /// Ends the application, the way the Exit softkey does (`CAknAppUi::Exit`).
    ///
    /// The framework unwinds on its own terms afterwards: the app UI destructor runs,
    /// which removes the view, deletes it, and then drops this application object.
    pub fn exit(&self) {
        // SAFETY: `CAknAppUi::Exit` is a non-leaving member reached through the shim,
        // and `self.app_ui` is the app UI that owns this object.
        unsafe { symrs_app_ui_exit(self.app_ui) }
    }

    /// The `CShimAppUi*` the shim passed to `construct`.
    ///
    /// A component that is a control in its own right — [`crate::List`] is the first —
    /// needs it twice over: to put itself on the control stack, and to ask for
    /// `ClientRect()`, which is the only honest source of a full-screen rectangle (the
    /// view's own `Rect()` is window-relative and reports an origin of `(0, 0)`).
    /// Handing the raw pointer out is safe; using it is not, which is why this is
    /// `pub(crate)` and every call site carries a `// SAFETY:` note.
    pub(crate) const fn app_ui(&self) -> *mut c_void {
        self.app_ui
    }
}
