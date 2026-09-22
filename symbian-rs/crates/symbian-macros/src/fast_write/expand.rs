//! The Rust a fast `write!` expands to, as text with holes.
//!
//! The text is built here, where a host `#[test]` can read it; [`super`] parses it
//! into tokens and fills each `@hole` with tokens of the caller's (the destination,
//! the arguments, the crate path), so their spans — and so every diagnostic about
//! them — stay the caller's.
//!
//! The shape, for `write!(w, "n={}", n)`:
//!
//! ```text
//! {
//!     use $crate::__private::{Enter as _, SinkKind as _, WriteKind as _, SlowKind as _};
//!     w.__symbian_fmt_enter((&n,), |__d, (__a0,)| {
//!         <one write per piece>
//!         Ok(())
//!     })
//! }
//! ```
//!
//! `w` is evaluated once, by one method call with `w` as its receiver, exactly as
//! `core::write!`'s `w.write_fmt(…)`: the same auto-referencing, the same two-phase
//! borrow (so `write!(s, "{}", s.len())` still compiles), and the arguments evaluated
//! after it, each once, in `format_args!`'s order.
//!
//! Each write asks a `Probe` of the destination's and the argument's types for its
//! kind. Autoref specialisation picks the most specific kind that applies — a
//! destination with a native `Sink`, then any `fmt::Write`, then the slow kind — and
//! the slow kind calls the closure written here, at the call site, which formats that
//! one piece through `write_fmt(format_args!("{}", a))`: the same trait the caller's
//! own `write!` would have found, with the same result type.

use super::plan::{Piece, Plan};

/// The expansion of `plan`. Holes: `@krate`, `@dst`, `@slot0`… for the values, and
/// `@view0`… for the name the slow closure gives each value — an identifier carrying
/// the argument's own span, so that "`T` doesn't implement `Display`" points at the
/// argument, as it does for `core::write!`, and not at the whole invocation.
pub fn expansion(plan: &Plan) -> String {
    let slots: String = (0..plan.slots.len())
        .map(|i| format!("@slot{i}, "))
        .collect();
    let names: String = (0..plan.slots.len()).map(|i| format!("__a{i}, ")).collect();
    let mut out = format!(
        "{{ use @krate::__private::{{Enter as _, SinkKind as _, WriteKind as _, SlowKind as _}}; \
         (@dst).__symbian_fmt_enter(({slots}), |__d, ({names})| {{ "
    );
    for piece in &plan.pieces {
        let (value, view) = match piece {
            Piece::Text(text) => (format!("{text:?}"), String::from("a")),
            Piece::Arg(slot) => (format!("__a{slot}"), format!("@view{slot}")),
        };
        out.push_str(&format!(
            "match (&&&@krate::__private::Probe::of(&*__d, {value})).__symbian_kind()\
             .put(&mut *__d, {value}, |d, {view}| d.write_fmt(::core::format_args!(\"{{}}\", {view}))) {{ \
             ::core::result::Result::Ok(()) => {{}} \
             ::core::result::Result::Err(e) => return ::core::result::Result::Err(e), }} "
        ));
    }
    out.push_str("::core::result::Result::Ok(()) }) }");
    out
}
