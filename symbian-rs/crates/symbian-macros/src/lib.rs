//! The attribute behind `symbian_std::main`.
//!
//! This crate is an implementation detail: an application depends on `symbian-std`
//! and writes `#[symbian_std::main]`, which is the re-export of [`macro@main`] here.
//! It has no dependencies of its own — not `syn`, not `quote` — because the grammar
//! it has to read is one function header, and because the logic then stays a pure
//! `&str` function that ordinary `#[test]`s can drive: the `proc_macro` API panics
//! the moment it is touched outside a real expansion, so a test can never build a
//! `TokenStream`.
//!
//! A proc macro is compiled for the *host* even though every other crate in this
//! workspace is compiled for `arm-symbian-e32` with `-Zbuild-std`; cargo does that on
//! its own, with no change to the target JSON or the build flags (experiment 81).

use proc_macro::TokenStream;

mod cursor;
mod entry;
mod signature;

#[cfg(test)]
mod tests;

use entry::Entry;

/// Declares a function as the application's entry point.
///
/// ```ignore
/// #![no_std]
///
/// use symbian_std::prelude::*;
///
/// #[symbian_std::main]
/// fn main() -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// The attribute writes the `extern "C"` function `eexe.lib`'s startup calls — the
/// C++-mangled `E32Main()`, `_Z7E32Mainv`, returning `TInt` — and converts whatever
/// `main` returns through `IntoExitCode`. `()` is `KErrNone`, an `i32` is itself, and
/// a `Result` is `Ok`'s code or the error's, so `?` works all the way out of `main`.
///
/// The function must be called `main`, take no arguments, and return something that
/// implements `IntoExitCode`. Nothing else about it is constrained: the body is
/// re-emitted exactly as written.
///
/// # The second shape
///
/// `#[symbian_std::main(gui)]` is reserved for an Avkon application (step 75) and is
/// refused until it exists. It cannot be the same generated code: a console
/// application creates and runs its own `CActiveScheduler`, while an Avkon one must
/// not, because CONE creates and runs `CCoeScheduler` and `CCoeEnv` is itself a
/// `CActive` on it. `gui` will therefore hand the process to the framework instead of
/// running `main` to completion, and `main` will become the place the application is
/// built rather than the place it ends.
#[proc_macro_attribute]
pub fn main(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let generated = match Entry::parse(&attribute.to_string(), &item.to_string()) {
        Ok(entry) => entry.wrapper(),
        Err(message) => compile_error(&message),
    };
    // The user's function is passed through as the token stream it arrived as, so a
    // diagnostic about its body still points at the body. The generated wrapper is
    // the only thing this attribute writes, and its tokens carry the call site — the
    // `#[symbian_std::main]` line — as their span.
    let mut out = tokens(&generated);
    out.extend(item);
    out
}

/// Rust source this crate wrote itself, back as tokens. The input is generated here
/// and always lexes; an empty stream rather than a panic is the honest answer if that
/// ever stops being true, and the user's own item is still emitted after it.
fn tokens(source: &str) -> TokenStream {
    source.parse().unwrap_or_else(|_| TokenStream::new())
}

/// `compile_error!("…")`, so the message lands on the attribute the user wrote.
fn compile_error(message: &str) -> String {
    format!("::core::compile_error!(\"{}\");\n", escape(message))
}

fn escape(message: &str) -> String {
    message.replace('\\', "\\\\").replace('"', "\\\"")
}
