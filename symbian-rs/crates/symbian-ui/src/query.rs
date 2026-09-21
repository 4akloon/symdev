//! `query`: Avkon's modal prompts — ask the user for text or a number.
//!
//! A query is the one piece of Avkon that fits in a plain function call. It is modal and
//! runs its own loop: `CAknQueryDialog::NewL(…)->ExecuteLD(…)` shows itself, takes over
//! the keypad, and returns only when the user confirms or cancels. So it needs no place
//! on the control stack, no observer and no state in the application struct, and this
//! module is three free functions rather than a type:
//!
//! ```ignore
//! fn key(&mut self, event: KeyEvent, ui: &Ui) -> KeyResponse {
//!     if event.code == key::SELECT {
//!         if let Ok(Some(name)) = query::text("Name?", 64) {
//!             self.name = name;
//!             ui.redraw();
//!         }
//!         return KeyResponse::Consumed;
//!     }
//!     KeyResponse::NotConsumed
//! }
//! ```
//!
//! `Ok(None)` is the user cancelling — an ordinary outcome, not an error. `Err` is the
//! framework failing: a leave out of `ExecuteLD`, or [`ErrorKind::NotReady`] if a query
//! is asked for before CONE has an environment.
//!
//! # Where the maximum length comes from
//!
//! [`text`] takes it. `CAknTextQueryDialog` writes into a descriptor the *caller* owns
//! and uses that descriptor's maximum length as the bound on what may be typed, so some
//! number has to be chosen, and choosing it here — 256, say — would be an invented limit
//! the application could neither see nor change. It is an argument instead, checked
//! against [`MAX_TEXT_LEN`], and the buffer is allocated for exactly that many UTF-16
//! code units.
//!
//! # No `confirm`
//!
//! There is deliberately no `query::confirm`. `ExecuteLD` needs a resource id, and the
//! ROM's own `avkon.rsg` has one for each *data* query (`R_AVKON_DIALOG_QUERY_VALUE_TEXT`
//! and friends) and **none for a confirmation query**: `AVKON_CONFIRMATION_QUERY` is only
//! a resource STRUCT in `avkon.rh`, a shape an application instantiates in its own
//! `.rss`. Until symdev generates such a resource and can tell the shim its id, a
//! confirmation query would be a guessed resource id, which is not a thing this SDK
//! ships. The requirement, with the resource text it needs, is written out at the foot
//! of `symbian-rs/shims/s60/symrs_query.cpp`.
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use symbian_core::des::{decode_utf16_into, encode_utf16_into, utf16_len};
use symbian_core::{ErrorKind, Result, SymbianError};

/// The longest prompt and the largest answer a query on this surface will take, in
/// UTF-16 code units.
///
/// It is a sanity bound on the allocation, not an Avkon limit: the editor a data query
/// puts on a 240×320 screen is a few lines, and asking for a megabyte of input is a
/// mistake worth naming as [`ErrorKind::Argument`] rather than honouring. What Avkon
/// itself does with a very long maximum was not observed.
pub const MAX_TEXT_LEN: usize = 1024;

/// `symrs_query_*` returned: the user confirmed the dialog.
const CONFIRMED: i32 = 1;

// The three entry points of `symbian-rs/shims/s60/symrs_query.cpp`. Each one is a
// complete `TRAP` unit around the leaving `CAknQueryDialog::NewL` and `ExecuteLD`, and
// returns 1 for confirmed, 0 for cancelled, or a negative `TInt`. Nothing throws while
// a Rust frame is on the stack — the rule of `avkon-rust-spec.md` §4.3.
unsafe extern "C" {
    /// `aText` is the caller's buffer; its `aTextMax` is what bounds the input, and
    /// `*aTextLength` comes back as the number of code units the user entered.
    fn symrs_query_text(
        prompt: *const u16,
        prompt_length: i32,
        text: *mut u16,
        text_max: i32,
        text_length: *mut i32,
    ) -> i32;

    /// `*value` is the number shown when the dialog opens and the number entered when
    /// this returns `CONFIRMED`.
    fn symrs_query_number(prompt: *const u16, prompt_length: i32, value: *mut i32) -> i32;
}

/// Asks the user for a line of text.
///
/// `max_len` is the greatest number of UTF-16 code units the user may enter, and is also
/// how large a buffer this allocates. `Ok(None)` means the user cancelled.
///
/// Fails with [`ErrorKind::Argument`] for a `max_len` of zero or above [`MAX_TEXT_LEN`],
/// [`ErrorKind::Overflow`] for a prompt longer than [`MAX_TEXT_LEN`],
/// [`ErrorKind::NotReady`] when there is no CONE environment yet, and otherwise with
/// whatever `ExecuteLD` left with.
///
/// A UTF-16 code unit is not a character: text outside the Basic Multilingual Plane
/// costs two units, so `max_len` bounds units and the `String` that comes back may hold
/// fewer characters than that.
pub fn text(prompt: &str, max_len: usize) -> Result<Option<String>> {
    if max_len == 0 || max_len > MAX_TEXT_LEN {
        return Err(SymbianError::of(ErrorKind::Argument));
    }
    let prompt = encode_prompt(prompt)?;
    let mut buffer: Vec<u16> = vec![0; max_len];
    let mut length: i32 = 0;
    // SAFETY: `prompt` and `buffer` are live for the call and their lengths are the
    // ones passed, both of which fit an `i32` because both are at most `MAX_TEXT_LEN`.
    // The shim wraps them in a `TPtrC16` and a `TPtr16` that it does not keep, writes
    // at most `max_len` code units into `buffer`, and traps every leave.
    let answer = unsafe {
        symrs_query_text(
            prompt.as_ptr(),
            prompt.len() as i32,
            buffer.as_mut_ptr(),
            max_len as i32,
            &mut length,
        )
    };
    if answer != CONFIRMED {
        return finish_cancelled(answer);
    }
    // The shim reports what `TDes16::Length()` said, but the buffer is this side's, so
    // a length outside it is clamped rather than trusted.
    let length = (length.max(0) as usize).min(max_len);
    Ok(Some(decode(&buffer[..length])?))
}

/// Asks the user for a whole number, with `initial` shown in the field.
///
/// `Ok(None)` means the user cancelled. The errors are [`text`]'s, less the ones about
/// a length.
pub fn number(prompt: &str, initial: i32) -> Result<Option<i32>> {
    let prompt = encode_prompt(prompt)?;
    let mut value = initial;
    // SAFETY: as `text`. `value` is a live `i32` the shim reads once and writes only
    // when the dialog was confirmed.
    let answer = unsafe { symrs_query_number(prompt.as_ptr(), prompt.len() as i32, &mut value) };
    if answer != CONFIRMED {
        return finish_cancelled(answer);
    }
    Ok(Some(value))
}

/// A prompt as UTF-16, bounded so the `as i32` below cannot be a surprise.
fn encode_prompt(prompt: &str) -> Result<Vec<u16>> {
    let len = utf16_len(prompt);
    if len > MAX_TEXT_LEN {
        return Err(SymbianError::of(ErrorKind::Overflow));
    }
    let mut out: Vec<u16> = vec![0; len];
    let written = encode_utf16_into(prompt, &mut out)?;
    out.truncate(written);
    Ok(out)
}

/// The shared tail of a query that did not come back confirmed: zero is the user
/// cancelling, a negative code is the framework failing, and a positive code other
/// than [`CONFIRMED`] cannot happen — the shim collapses every button id to one.
fn finish_cancelled<T>(answer: i32) -> Result<Option<T>> {
    match answer {
        0 => Ok(None),
        code if code < 0 => Err(SymbianError::from_code(code)),
        _ => Err(SymbianError::of(ErrorKind::Corrupt)),
    }
}

/// UTF-16 code units as a `String`.
///
/// The buffer is what the user typed, so it is at most `max_len` units, and UTF-8 needs
/// at most three bytes per unit — a surrogate pair is two units and four bytes.
fn decode(units: &[u16]) -> Result<String> {
    let mut bytes: Vec<u8> = vec![0; units.len() * 3];
    let text = decode_utf16_into(units, &mut bytes)?;
    Ok(String::from(text))
}
