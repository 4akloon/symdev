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
use symbian_core::{ErrorKind, Result, SymbianError, check};

use crate::ui::Ui;

mod rows;

pub use rows::{MAX_ITEM_TEXT, Rows};

use rows::{
    CALLBACKS, Owner, selected, set_items, symrs_list_count, symrs_list_create, symrs_list_destroy,
    symrs_list_set_selected,
};

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
        let owner = Box::into_raw(Box::new(Owner {
            raw: core::ptr::null_mut(),
            on_select: None,
        }));
        let mut raw: *mut c_void = core::ptr::null_mut();
        // SAFETY: `ui.app_ui()` is the `CShimAppUi*` the shim passed to `construct`,
        // alive for as long as the application object; `CALLBACKS` is a `'static` table
        // whose `size` word the shim checks; `owner` is the box just leaked and is freed
        // only in `Drop`, after `symrs_list_destroy`. The list takes its rectangle from
        // that app UI's `ClientRect()`, so no coordinate crosses this call.
        let err = unsafe {
            symrs_list_create(
                ui.app_ui(),
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
        // SAFETY: the box is alive and unborrowed — C++ has the pointer but calls
        // nothing through it until the list reports an event, which cannot happen
        // before this function has returned the list to the application.
        unsafe { (*owner).raw = raw };
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
        set_items(self.raw, items)
    }

    /// The highlighted row's index, or 0 when the list is empty.
    pub fn selected(&self) -> usize {
        selected(self.raw)
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
    /// reach it; what it is given instead is [`Rows`], which can rewrite the list. On a
    /// full-screen list that is the only surface a selection can show itself on, since
    /// the list covers the application's own view.
    pub fn on_select(&mut self, on_select: impl FnMut(usize, &mut Rows<'_>) + 'static) {
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
