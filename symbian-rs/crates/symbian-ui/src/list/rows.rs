//! `Rows`, the boundary itself, and the two operations both sides of it share.
//!
//! Everything that touches a raw handle lives here, so that [`super::List`] is only the
//! owning type: the `extern "C"` declarations of `shims/s60/symrs_list.h`, the callback
//! table Rust hands the shim, the box the shim calls back through, and the borrowed
//! `Rows` a callback is given in place of the `&mut List` it cannot have.
use core::ffi::c_void;
use core::marker::PhantomData;

use alloc::boxed::Box;
use symbian_core::des::encode_utf16_into;
use symbian_core::{Result, check};

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
pub(super) struct Callbacks {
    size: u32,
    selected: extern "C" fn(*mut c_void, i32) -> i32,
}

/// The application's "an item was chosen" callback: the index, and a handle that can
/// rewrite the rows.
pub(super) type OnSelect = Box<dyn FnMut(usize, &mut Rows<'_>)>;

/// What the shim's opaque `aOwner` really points at. Boxed once, so its address is
/// stable for as long as C++ holds it.
pub(super) struct Owner {
    /// The same handle [`super::List`] holds, so that the callback can rewrite the rows. It
    /// is a copy, not a second owner: only [`super::List`]'s `Drop` destroys the control.
    pub(super) raw: *mut c_void,
    pub(super) on_select: Option<OnSelect>,
}

/// The rows of a list, borrowed for the length of a callback.
///
/// A full-screen list covers the view, so the only place a selection can show itself is
/// the list — which means the callback has to be able to write to the thing that owns
/// it. Handing it a `Rows` instead of a `&mut List` is what makes that possible without
/// an `Rc`/`Weak` cycle in every application that wants it: `Rows` borrows the control
/// handle alone, never the closure beside it.
pub struct Rows<'a> {
    raw: *mut c_void,
    life: PhantomData<&'a mut ()>,
}

impl Rows<'_> {
    /// Replaces every row. See [`super::List::set_items`], which is this call.
    pub fn set_items(&mut self, items: &[&str]) -> Result<()> {
        set_items(self.raw, items)
    }

    /// The highlighted row's index, or 0 when the list is empty.
    pub fn selected(&self) -> usize {
        selected(self.raw)
    }
}

/// The `selected` callback, called from the list's own `OfferKeyEventL`.
///
/// Nothing here may unwind: a Rust panic is `abort` and a C++ exception crossing this
/// frame ends the process with no diagnostic. The application object is **not** borrowed
/// while this runs — the list is the control the framework called, not the view — so the
/// closure can touch anything it captured without aliasing an outer `&mut`.
///
/// The closure is **moved out** of the `Owner` for the length of the call and put back
/// afterwards. That costs two pointer writes and buys the one thing that cannot be
/// proved from the headers: whether anything the closure does can make the list report
/// a second event before the first has returned. If it ever does, the nested call finds
/// no closure and does nothing, instead of taking a second `&mut` to the same `Owner`.
extern "C" fn selected_thunk(owner: *mut c_void, index: i32) -> i32 {
    if owner.is_null() || index < 0 {
        return 0;
    }
    let owner = owner.cast::<Owner>();
    // SAFETY: `owner` is the `Box<Owner>` leaked in `super::List::new` for this list; the shim
    // passes back exactly that pointer, and never after `symrs_list_destroy`, which
    // `Drop` calls before the box is reclaimed. The framework is single-threaded, and
    // this borrow ends on the next line — before the closure runs — so a re-entrant call
    // cannot overlap it.
    let (raw, mut on_select) = unsafe { (&mut *owner).take() };
    if let Some(on_select) = on_select.as_mut() {
        let mut rows = Rows {
            raw,
            life: PhantomData,
        };
        on_select(index as usize, &mut rows);
    }
    // SAFETY: as above, and the slot is still `None`: only this function ever takes the
    // closure out, and a nested call would have put nothing back.
    unsafe { (&mut *owner).on_select = on_select };
    0
}

impl Owner {
    fn take(&mut self) -> (*mut c_void, Option<OnSelect>) {
        (self.raw, self.on_select.take())
    }
}

pub(super) static CALLBACKS: Callbacks = Callbacks {
    size: size_of::<Callbacks>() as u32,
    selected: selected_thunk,
};

unsafe extern "C" {
    pub(super) fn symrs_list_create(
        app_ui: *mut c_void,
        callbacks: *const Callbacks,
        owner: *mut c_void,
        out: *mut *mut c_void,
    ) -> i32;
    pub(super) fn symrs_list_destroy(list: *mut c_void);
    pub(super) fn symrs_list_clear(list: *mut c_void);
    pub(super) fn symrs_list_add(list: *mut c_void, text: *const u16, len: i32) -> i32;
    pub(super) fn symrs_list_commit(list: *mut c_void) -> i32;
    pub(super) fn symrs_list_count(list: *mut c_void) -> i32;
    pub(super) fn symrs_list_selected(list: *mut c_void) -> i32;
    pub(super) fn symrs_list_set_selected(list: *mut c_void, index: i32) -> i32;
}

/// The rows, behind either a [`super::List`] or a [`Rows`]. One implementation, because the
/// two differ only in what else they are allowed to touch.
pub(super) fn set_items(raw: *mut c_void, items: &[&str]) -> Result<()> {
    // SAFETY: `raw` is the handle `symrs_list_create` wrote, destroyed only by
    // `super::List`'s `Drop`, and a `Rows` cannot outlive the callback it was made for.
    // `Reset` on the item array is non-leaving.
    unsafe { symrs_list_clear(raw) };
    for item in items {
        // The shim writes the column separators, because the row format belongs to the
        // list style and not to the application: for `list_single_pane` a row is
        // `"\tLabel"`, the leading empty column being where a graphic style would put
        // its icon index (`aknlists.h`).
        let mut units = [0u16; MAX_ITEM_TEXT];
        let len = encode_utf16_into(item, &mut units)?;
        // SAFETY: `units` is a live stack array of `MAX_ITEM_TEXT` elements and
        // `len <= MAX_ITEM_TEXT`; the shim wraps the pair in a `TPtrC16`, copies it
        // into the item array and does not keep the pointer beyond the call.
        check(unsafe { symrs_list_add(raw, units.as_ptr(), len as i32) })?;
    }
    // SAFETY: as above; the shim TRAPs `HandleItemAdditionL` and returns its code.
    check(unsafe { symrs_list_commit(raw) }).map(drop)
}

pub(super) fn selected(raw: *mut c_void) -> usize {
    // SAFETY: `CurrentItemIndex` is a non-leaving `const` member reached through the
    // shim, and `raw` is live for as long as its holder is.
    let index = unsafe { symrs_list_selected(raw) };
    if index < 0 { 0 } else { index as usize }
}
