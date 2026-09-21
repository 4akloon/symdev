//! `List`: the S60 list box, which is the central component of an Avkon application.
//!
//! The Symbian side is `CAknSingleStyleListBox` — the `list_single_pane` style, the
//! simplest one that shows text and nothing else — built, stacked and owned by
//! `symbian-rs/shims/s60/symrs_list.cpp`. An application names no Symbian type and
//! writes no `unsafe`:
//!
//! ```ignore
//! let mut list = List::new(ui, &["Inbox", "Drafts", "Sent"])?;
//! list.on_select(move |index| { /* … */ });
//! ```
//!
//! # Who handles the keys
//!
//! The list goes on the control stack **above** the application's view, at
//! `ECoeStackPriorityDefault + 1`, and `coeaui.h` states the rule: a control with a
//! higher priority is offered a key first. So the list consumes Up and Down and the
//! selection key itself, and the application's [`crate::App::key`] sees only what the
//! list did not want. That is the intended division — an application reacts to a
//! *selection*, not to an arrow — and it is a documented priority rather than the order
//! two `AddToStackL` calls happened to run in.
//!
//! # What the shim owns
//!
//! The item array (`CDesCArrayFlat`) belongs to the shim, which hands the list's model a
//! borrow of it (`ELbmDoesNotOwnItemArray`) and frees it in its own destructor.
//! [`List::set_items`] refills that one array rather than replacing it, so no ownership
//! ever crosses the boundary and `SetItemTextArray`'s non-deleting assignment can neither
//! leak nor double-free. Dropping a `List` takes the control off the stack and destroys
//! both.
use core::ffi::c_void;

use alloc::boxed::Box;
use symbian_core::des::encode_utf16_into;
use symbian_core::{ErrorKind, Result, SymbianError, check};

use crate::ui::Ui;

/// The greatest number of UTF-16 code units one row's label may hold.
///
/// A row is one line of a 240-pixel-wide screen, so this is far past what can be read;
/// text longer than it is an [`ErrorKind::Overflow`] rather than a silent truncation,
/// because unlike `Gc::text` this call has somewhere to report a failure.
pub const MAX_ITEM_TEXT: usize = 128;

/// The Rust half of `SymRsListCallbacks` in `shims/s60/symrs_list.h`.
///
/// It is a table of function pointers handed over at create time rather than a symbol
/// the shim imports, which is why adding a list costs the link line nothing: the one
/// call that runs from C++ into Rust needs no `-u` (experiment 86 needed one for
/// `symrs_app_vtbl` and this deliberately avoids a second).
#[repr(C)]
struct Callbacks {
    size: u32,
    selected: extern "C" fn(*mut c_void, i32) -> i32,
}

/// What the shim's opaque `aOwner` really points at. Boxed once, so its address is
/// stable for as long as C++ holds it.
struct Owner {
    on_select: Option<Box<dyn FnMut(usize)>>,
}

/// The `selected` callback, called from the list's own `OfferKeyEventL`.
///
/// Nothing here may unwind: a Rust panic is `abort` and a C++ exception crossing this
/// frame ends the process with no diagnostic. The application object is **not** borrowed
/// while this runs — the list is the control the framework called, not the view — so the
/// closure can touch anything it captured without aliasing an outer `&mut`.
extern "C" fn selected_thunk(owner: *mut c_void, index: i32) -> i32 {
    if owner.is_null() || index < 0 {
        return 0;
    }
    // SAFETY: `owner` is the `Box<Owner>` leaked in `List::new` for this list; the shim
    // passes back exactly that pointer and never after `symrs_list_destroy`, which
    // `Drop` calls before the box is reclaimed. The framework is single-threaded, and
    // no other borrow of this `Owner` is live: `List`'s own methods never take one.
    let owner = unsafe { &mut *owner.cast::<Owner>() };
    if let Some(on_select) = owner.on_select.as_mut() {
        on_select(index as usize);
    }
    0
}

static CALLBACKS: Callbacks = Callbacks {
    size: size_of::<Callbacks>() as u32,
    selected: selected_thunk,
};

unsafe extern "C" {
    fn symrs_list_create(
        app_ui: *mut c_void,
        view: *mut c_void,
        callbacks: *const Callbacks,
        owner: *mut c_void,
        out: *mut *mut c_void,
    ) -> i32;
    fn symrs_list_destroy(list: *mut c_void);
    fn symrs_list_clear(list: *mut c_void);
    fn symrs_list_add(list: *mut c_void, text: *const u16, len: i32) -> i32;
    fn symrs_list_commit(list: *mut c_void) -> i32;
    fn symrs_list_count(list: *mut c_void) -> i32;
    fn symrs_list_selected(list: *mut c_void) -> i32;
    fn symrs_list_set_selected(list: *mut c_void, index: i32) -> i32;
}

/// A list box filling the application's view.
///
/// Build one in [`crate::App::construct`] and keep it in the application struct: it is
/// a live control on the control stack, and dropping it removes it.
pub struct List {
    raw: *mut c_void,
    owner: *mut Owner,
}

impl List {
    /// Builds the list and puts it on the control stack, showing `items`.
    ///
    /// The first item is selected. An error here is the framework's own
    /// (`KErrNoMemory` and the like) and reaches the application as a leave only after
    /// its frame has returned.
    pub fn new(ui: &Ui, items: &[&str]) -> Result<Self> {
        let owner = Box::into_raw(Box::new(Owner { on_select: None }));
        let mut raw: *mut c_void = core::ptr::null_mut();
        // SAFETY: `ui`'s two handles are the `CShimAppUi*` and `CShimView*` the shim
        // passed to `construct`, both alive for as long as the application object;
        // `CALLBACKS` is a `'static` table whose `size` word the shim checks; `owner`
        // is the box just leaked and is freed only in `Drop`, after `symrs_list_destroy`.
        let err = unsafe {
            symrs_list_create(
                ui.app_ui(),
                ui.view(),
                &raw const CALLBACKS,
                owner.cast(),
                &raw mut raw,
            )
        };
        if err != 0 || raw.is_null() {
            // SAFETY: `owner` came from `Box::into_raw` a few lines up and the shim
            // reported that it kept nothing, so this is the only pointer to it.
            drop(unsafe { Box::from_raw(owner) });
            // A zero return with a null handle cannot happen — the shim writes one or
            // the other — but it must still be an error and not a null dereference.
            return Err(match err {
                0 => ErrorKind::General.into(),
                err => SymbianError::from_code(err),
            });
        }
        let mut list = Self { raw, owner };
        list.set_items(items)?;
        Ok(list)
    }

    /// Replaces every row. The highlight is kept where it was, clamped into the new
    /// range; an empty list has no current item.
    ///
    /// A label longer than [`MAX_ITEM_TEXT`] code units is an [`ErrorKind::Overflow`],
    /// and the list is left holding whatever was appended before it — call it again
    /// with a shorter label rather than reading the half-filled list.
    pub fn set_items(&mut self, items: &[&str]) -> Result<()> {
        // SAFETY: `self.raw` is the handle `symrs_list_create` wrote and is destroyed
        // only in `Drop`. `Reset` on the item array is non-leaving.
        unsafe { symrs_list_clear(self.raw) };
        for item in items {
            self.push(item)?;
        }
        // SAFETY: as above; the shim TRAPs `HandleItemAdditionL` and returns its code.
        check(unsafe { symrs_list_commit(self.raw) }).map(drop)
    }

    /// Appends one row, without telling the list yet. The shim writes the column
    /// separators, because the row format belongs to the list style: for
    /// `list_single_pane` a row is `"\tLabel"`, the leading empty column being where a
    /// graphic style would put its icon index.
    fn push(&mut self, item: &str) -> Result<()> {
        let mut units = [0u16; MAX_ITEM_TEXT];
        let len = encode_utf16_into(item, &mut units)?;
        // SAFETY: `units` is a live stack array of `MAX_ITEM_TEXT` elements and
        // `len <= MAX_ITEM_TEXT`; the shim wraps the pair in a `TPtrC16`, copies it into
        // the item array and does not keep the pointer beyond the call.
        check(unsafe { symrs_list_add(self.raw, units.as_ptr(), len as i32) }).map(drop)
    }

    /// The highlighted row's index, or 0 when the list is empty.
    pub fn selected(&self) -> usize {
        // SAFETY: `CurrentItemIndex` is a non-leaving `const` member reached through the
        // shim, and `self.raw` is live.
        let index = unsafe { symrs_list_selected(self.raw) };
        if index < 0 { 0 } else { index as usize }
    }

    /// Moves the highlight and redraws. An index past the end is an
    /// [`ErrorKind::Argument`].
    pub fn set_selected(&mut self, index: usize) -> Result<()> {
        let Ok(index) = i32::try_from(index) else {
            return Err(ErrorKind::Argument.into());
        };
        // SAFETY: `SetCurrentItemIndexAndDraw` is non-leaving; the shim range-checks the
        // index against the item array before calling it.
        check(unsafe { symrs_list_set_selected(self.raw, index) }).map(drop)
    }

    /// How many rows the list holds.
    pub fn len(&self) -> usize {
        // SAFETY: a non-leaving read of the item array's count.
        let count = unsafe { symrs_list_count(self.raw) };
        if count < 0 { 0 } else { count as usize }
    }

    /// Whether the list holds no rows at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Runs `on_select` with the row's index whenever an item is chosen — the selection
    /// key, or a tap on a device that has a touch screen.
    ///
    /// The application object is not borrowed while this runs, so the closure cannot
    /// reach it: give it what it needs to record, for example an
    /// `alloc::rc::Rc<core::cell::Cell<usize>>` the application also holds.
    pub fn on_select(&mut self, on_select: impl FnMut(usize) + 'static) {
        // SAFETY: `self.owner` came from `Box::into_raw` in `new` and is alive until
        // `Drop`. `&mut self` is proof no other borrow is live — the only other one is
        // taken by `selected_thunk`, which the framework can only call between our
        // calls, never during one, because everything here runs on its single thread.
        let owner = unsafe { &mut *self.owner };
        owner.on_select = Some(Box::new(on_select));
    }
}

impl Drop for List {
    fn drop(&mut self) {
        // SAFETY: the handle is destroyed exactly once. `symrs_list_destroy` takes the
        // control off the stack and deletes it together with the item array, so nothing
        // in C++ holds `self.owner` afterwards and the box below is the last reference.
        unsafe { symrs_list_destroy(self.raw) };
        // SAFETY: `self.owner` came from `Box::into_raw` in `new` and is reclaimed here
        // for the first and only time, after the C++ side that borrowed it is gone.
        drop(unsafe { Box::from_raw(self.owner) });
    }
}
