//! The one UTF-16 encoder that cuts instead of failing, shared by every callback that
//! has no error channel: [`crate::Gc::text`] (inside `draw`) and [`crate::Menu`] (a
//! label, inside `DynInitMenuPaneL`).
#![forbid(unsafe_code)]

/// Encodes as many whole characters of `text` as fit in `out`, and returns how many
/// code units that was.
///
/// Whole characters: a cut between the halves of a surrogate pair would put a lone
/// surrogate in the descriptor. Nothing can overflow `out`, because a character is
/// only written once its width is known to fit.
///
/// Out of line on purpose (experiment 104): inlined into a `draw` it was ~800 bytes of
/// every application's image, next to a second, erroring encoder for the same job.
#[inline(never)]
pub(crate) fn encode_cut(text: &str, out: &mut [u16]) -> usize {
    let mut n = 0;
    for c in text.chars() {
        let Some(slot) = out.get_mut(n..n + c.len_utf16()) else {
            break;
        };
        n += c.encode_utf16(slot).len();
    }
    n
}
