//! Avkon notes: the four popups an application tells the user something with.
//!
//! ```ignore
//! use symbian_std::ui::note;
//!
//! note::info("Saved")?;
//! note::error("No network")?;
//! ```
//!
//! A note is the one piece of Avkon that needs nothing around it. It has no place on
//! the control stack of its own, no observer and no model: `ExecuteLD` builds the
//! dialog, shows it and destroys it, so there is nothing for an application to own.
//! That is why these are free functions and not a type an application has to keep.
//!
//! # What "modal" means here, measured
//!
//! **These four do not block.** `CEikDialog::ExecuteLD` (`eikdialg.h`) "returns
//! immediately unless `EEikDialogFlagWait` has been specified in the `DIALOG`
//! resource", and the resources behind the four classes below —
//! `R_AKN_INFORMATION_NOTE` and its three siblings — do not set it. Measured in EKA2L1
//! with `symbian_std::time::Instant` around the call rather than taken on trust; the
//! figure is in experiment 92.
//!
//! So [`info`] returns while the note is still on screen, the note dismisses itself
//! after its own timeout, and an application that shows two in a row shows the second
//! over the first. Avkon also declares a waiting form of each class
//! (`CAknInformationNote(ETrue)`, which picks `R_AKN_INFORMATION_NOTE_WAIT`); it is
//! **not** wrapped here, because nothing in this repository has yet needed a note that
//! blocks and a wrapper for it would have no observed behaviour behind it.
//!
//! # Where one may be shown
//!
//! Anywhere the application environment exists, which from the Rust side means
//! [`App::construct`](crate::App) onwards — the shim has already run `BaseConstructL`
//! by then. A note shown before there is a `CEikonEnv` is refused with
//! [`ErrorKind::NotReady`] rather than left to crash inside Avkon.
//!
//! Not from [`App::draw`](crate::App::draw): that callback is made outside a trap
//! harness and `ExecuteLD` leaves. The type system says so too — `draw` takes `&self`
//! and returns nothing, so it has no way to report the error this returns.
use symbian_core::des::encode_utf16_into;
use symbian_core::{Result, check};

/// The longest note text, in UTF-16 code units; longer is [`ErrorKind::Overflow`].
///
/// 256 is the one length the SDK names for note text: `TAknNoteResData::iText` in
/// `aknnotewrappers.h` is a `TBuf<256>`. That field is the resource-borne text and not
/// the descriptor these functions pass, so it is a **defensible bound, not a measured
/// one** — what Avkon does with a longer prompt was not observed. The buffer is on the
/// stack so that a note can be shown from a callback that must not allocate.
///
/// [`ErrorKind::Overflow`]: symbian_core::ErrorKind::Overflow
pub const MAX_NOTE_TEXT: usize = 256;

/// Which of Avkon's four notes to show.
///
/// The discriminants are the ABI: `TSymRsNoteKind` in
/// `symbian-rs/shims/s60/symrs_note.cpp` declares the same four values, and the shim
/// answers an unknown one with `KErrArgument` rather than picking a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Note {
    /// `CAknInformationNote` — something happened that the user did not ask about.
    Information = 0,
    /// `CAknConfirmationNote` — what the user asked for is done.
    Confirmation = 1,
    /// `CAknWarningNote` — it worked, but not the way the user probably wanted.
    Warning = 2,
    /// `CAknErrorNote` — it did not work.
    Error = 3,
}

impl Note {
    /// Shows this note and returns as soon as it is on screen.
    ///
    /// `Err` is the Symbian code the shim's `TRAP` caught, or
    /// [`ErrorKind::Overflow`](symbian_core::ErrorKind::Overflow) when `text` is longer
    /// than [`MAX_NOTE_TEXT`] code units, or
    /// [`ErrorKind::NotReady`](symbian_core::ErrorKind::NotReady) when there is no
    /// application environment to draw into.
    pub fn show(self, text: &str) -> Result<()> {
        let mut units = [0u16; MAX_NOTE_TEXT];
        let len = encode_utf16_into(text, &mut units)?;
        // SAFETY: the shim function is a complete `TRAP` unit around one `new (ELeave)`
        // and one `ExecuteLD`, so no C++ exception can reach this frame. It reads
        // exactly `len` code units from `units`, which is a live stack array of
        // `MAX_NOTE_TEXT` and `len <= MAX_NOTE_TEXT` because `encode_utf16_into` said
        // so; it wraps the pair in a `TPtrC16` that does not outlive the call.
        let code = unsafe { symrs_note_show(self as i32, units.as_ptr(), len as i32) };
        check(code).map(|_| ())
    }
}

/// `CAknInformationNote` — something happened that the user did not ask about.
pub fn info(text: &str) -> Result<()> {
    Note::Information.show(text)
}

/// `CAknConfirmationNote` — what the user asked for is done.
pub fn confirm(text: &str) -> Result<()> {
    Note::Confirmation.show(text)
}

/// `CAknWarningNote` — it worked, but not the way the user probably wanted.
pub fn warn(text: &str) -> Result<()> {
    Note::Warning.show(text)
}

/// `CAknErrorNote` — it did not work.
pub fn error(text: &str) -> Result<()> {
    Note::Error.show(text)
}

unsafe extern "C" {
    /// `symbian-rs/shims/s60/symrs_note.cpp`, TRAPped.
    ///
    /// `new (ELeave)` one of the four `CAknResourceNoteDialog` subclasses, then
    /// `CAknResourceNoteDialog::ExecuteLD(const TDesC16&)`
    /// (`_ZN22CAknResourceNoteDialog9ExecuteLDERK7TDesC16`, avkon.dso), which shows the
    /// note and destroys the dialog. Returns `KErrNone`, the leave code,
    /// `KErrArgument` for a bad kind or a null pointer, or `KErrNotReady` when
    /// `CEikonEnv::Static()` is null.
    fn symrs_note_show(kind: i32, text: *const u16, length: i32) -> i32;
}

/// The kind numbers are the ABI `TSymRsNoteKind` declares, and a reordering here would
/// silently turn every error note into an information one. Checked at compile time
/// rather than in a test, because every crate in this workspace is phone code and
/// `cargo test` never runs on this target.
const _: () = {
    assert!(Note::Information as i32 == 0);
    assert!(Note::Confirmation as i32 == 1);
    assert!(Note::Warning as i32 == 2);
    assert!(Note::Error as i32 == 3);
    // The length has to fit the `TInt` the shim takes.
    assert!(MAX_NOTE_TEXT > 0 && MAX_NOTE_TEXT <= i32::MAX as usize);
};
