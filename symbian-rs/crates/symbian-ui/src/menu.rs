//! `Menu`: the Options menu, declared by the application every time it opens.
//!
//! The menu is not in `symdev.toml` and there is no command id anywhere in an
//! application. `symdev.toml` holds what the phone needs *before* the application runs
//! — uid3, capabilities, vendor, caption, icon — and the Options menu is only ever
//! needed while it runs. So the pane is empty in the compiled resource and is filled
//! from Rust through `CEikMenuPane::AddMenuItemL`, which `eikmenup.h:456` documents as
//! adding an item "dynamically", each time `MEikMenuObserver::DynInitMenuPaneL` fires.
//!
//! ```ignore
//! fn menu(&self, m: &mut Menu<Self>) {
//!     m.item("More bars", |app| app.bars += 1);
//!     m.item("Reset", |app| app.bars = 3);
//!     m.exit("Exit");
//! }
//! ```
//!
//! # Why the action is a plain `fn`, and why that is what makes it sound
//!
//! [`App::menu`](crate::App::menu) takes `&self`, so the framework can be told a
//! different menu for a different state; but the action needs `&mut Self`, and the two
//! borrows may never be live together. They are not. The action is a
//! **non-capturing closure**, which coerces to a `fn(&mut A)` pointer — a `Copy` value
//! that borrows nothing. So the crate calls `menu(&self, …)` a second time when a
//! command arrives, copies the pointer of the chosen item out, **ends the `&self`
//! borrow**, and only then calls it with `&mut A`.
//!
//! The same shape answers re-entrancy: the action is handed the application and
//! nothing else — no [`Ui`](crate::Ui), no handle into C++ — so it cannot ask the
//! framework for anything, and the framework cannot call back into a Rust frame that
//! is not there. The repaint a menu action almost always wants is done by the crate
//! after the action has returned (`crate::vtbl`), which is also the one place it can
//! be done without handing the action a way back in.
use core::ffi::c_void;
use core::marker::PhantomData;

use crate::abi::symrs_menu_add;
use crate::utf16::encode_cut;

/// The greatest number of UTF-16 code units a menu label may carry.
///
/// It is `CEikMenuPaneItem::SData::ENominalTextLength` (`eikmenup.h:76`): `iText` is a
/// `TBuf<40>`, and `TDes16::Copy` of anything longer is a **descriptor panic**
/// (`ETDes16Overflow = 11`, `e32panic.h:131`) — not a leave, so no `TRAP` would catch
/// it and the application would simply die. A longer label is therefore cut here, on a
/// character boundary, before it can reach the descriptor at all: the same choice
/// [`Gc::text`](crate::Gc::text) makes, for the same reason, because a `menu` callback
/// has no error channel either.
pub const MAX_LABEL: usize = 40;

/// The first command number the items of a menu carry.
///
/// A menu item's number never reaches an application now — it is the item's position
/// in the order [`App::menu`](crate::App::menu) declared it, plus this — but the range
/// still has to miss everything the platform names. **Below `0x4000`** live
/// `EEikCmdExit = 0x100` (`eikon.hrh:376`), Avkon's `EAknSoftkey*` at 3000–3200 and its
/// reserved softkey ranges at `0x1000`/`0x1100`/`0x1200`; **at `0x8000`** `CBA_BUTTON`'s
/// `id`, a `WORD` in `eikon.rh`, would stop being representable.
const FIRST: i32 = 0x4000;

/// The greatest number of items one menu may declare, which is the rest of that range.
const LIMIT: u16 = 0x3fff;

/// `EEikCmdExit` (`eikon.hrh`), the command the shim ends the application on.
const EXIT: i32 = 0x100;

/// The Options menu being built, or being asked which action a command stands for.
///
/// It is borrowed for exactly one call of [`App::menu`](crate::App::menu) and cannot be
/// stored: the lifetime is the `CEikMenuPane*` the framework is showing.
pub struct Menu<'a, A> {
    purpose: Purpose<'a>,
    /// The position of the next item, which is also its command number.
    next: u16,
    /// The action [`Purpose::Find`] was looking for, once it has been passed.
    found: Option<fn(&mut A)>,
    /// The first failure `AddMenuItemL` reported, as a Symbian error code.
    error: i32,
}

#[derive(Clone, Copy)]
enum Purpose<'a> {
    /// Fill the pane the framework is about to put on the screen, which is live for
    /// the `'a` of the `menu` call.
    Fill(*mut c_void, PhantomData<&'a mut c_void>),
    /// Find the action of the item at this position; touch nothing.
    Find(u16),
}

impl<'a, A> Menu<'a, A> {
    /// # Safety
    ///
    /// `pane` is the `CEikMenuPane*` the shim passed to `menu`, valid for that call
    /// only.
    pub(crate) const unsafe fn fill(pane: *mut c_void) -> Self {
        Self {
            purpose: Purpose::Fill(pane, PhantomData),
            next: 0,
            found: None,
            error: 0,
        }
    }

    /// A menu that adds nothing and only remembers the action at `index`.
    pub(crate) const fn find(index: u16) -> Self {
        Self {
            purpose: Purpose::Find(index),
            next: 0,
            found: None,
            error: 0,
        }
    }

    /// One line of the menu: what is read, and what it does to the application.
    ///
    /// The action is called with the application itself, after this callback has
    /// returned, and the view is repainted afterwards. A label longer than
    /// [`MAX_LABEL`] code units is cut to it.
    pub fn item(&mut self, label: &str, action: fn(&mut A)) {
        let Some(index) = self.take_index() else {
            return;
        };
        match self.purpose {
            Purpose::Fill(pane, _) => self.add(pane, label, FIRST + index as i32),
            Purpose::Find(want) => {
                if want == index {
                    self.found = Some(action);
                }
            }
        }
    }

    /// A line that ends the application, the way the Exit softkey does.
    ///
    /// It carries `EEikCmdExit`, which is the platform's own name for it and which the
    /// shim acts on itself, so no Rust frame is on the stack while the framework tears
    /// the application down. There is deliberately no closure: an action is handed the
    /// application and nothing else, and ending the application is not something it
    /// can do to itself.
    pub fn exit(&mut self, label: &str) {
        // The position is spent either way, so that `Fill` and `Find` count alike.
        if self.take_index().is_none() {
            return;
        }
        if let Purpose::Fill(pane, _) = self.purpose {
            self.add(pane, label, EXIT);
        }
    }

    /// The position of the item being declared, or `None` past [`LIMIT`].
    fn take_index(&mut self) -> Option<u16> {
        if self.next >= LIMIT {
            return None;
        }
        let index = self.next;
        self.next += 1;
        Some(index)
    }

    fn add(&mut self, pane: *mut c_void, label: &str, command: i32) {
        let mut units = [0u16; MAX_LABEL];
        let len = encode_cut(label, &mut units);
        // SAFETY: `symrs_menu_add` traps `AddMenuItemL` in the shim and returns its error,
        // so nothing leaves across this frame. `pane` is the pane the framework is
        // showing for this call, and `units` is a live stack array of `MAX_LABEL`
        // elements with `len <= MAX_LABEL`; the shim copies out of it and keeps
        // nothing.
        let err = unsafe { symrs_menu_add(pane, units.as_ptr(), len as i32, command) };
        if self.error == 0 {
            self.error = err;
        }
    }

    /// The action of the item that was looked for, once the application has declared
    /// its menu. It borrows nothing, so the `&self` the declaration needed is free.
    pub(crate) const fn action(&self) -> Option<fn(&mut A)> {
        self.found
    }

    /// The first `AddMenuItemL` failure, for the shim to turn into a leave **after**
    /// this frame has returned.
    pub(crate) const fn error(&self) -> i32 {
        self.error
    }
}

/// The position the command `raw` stands for, or `None` when it is not a menu item's
/// — a softkey the framework did not consume, or a platform command.
pub(crate) const fn index_of(raw: i32) -> Option<u16> {
    let index = raw - FIRST;
    if index >= 0 && index < LIMIT as i32 {
        Some(index as u16)
    } else {
        None
    }
}

/// The numbering, asserted where it is written. These are `const` assertions rather
/// than `#[test]`s because this workspace builds for the phone and `cargo test` never
/// runs in it — a compile failure is the only failure it can have.
const _: () = {
    assert!(index_of(0x4000).is_some());
    assert!(index_of(EXIT).is_none());
    assert!(index_of(3000).is_none());
    assert!(index_of(0x3fff).is_none());
    assert!(index_of(0x7ffe).is_some());
    assert!(index_of(0x7fff).is_none());
    assert!(FIRST + LIMIT as i32 <= 0x8000);
};
