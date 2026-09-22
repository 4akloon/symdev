//! What the attribute accepts and what it says when it refuses.
//!
//! The inputs are what `TokenStream::to_string` renders, which is why the fixtures
//! are spaced the way a token stream prints (`Result < () >`, `#[doc = " …"]`) rather
//! than the way a person writes them: that is exactly what the macro will see.

use crate::entry::{E32MAIN, Entry};

fn wrapper(arguments: &str, item: &str) -> String {
    match Entry::parse(arguments, item) {
        Ok(entry) => entry.wrapper(),
        Err(message) => panic!("expected an entry point, got: {message}"),
    }
}

fn refusal(arguments: &str, item: &str) -> String {
    match Entry::parse(arguments, item) {
        Ok(_) => panic!("expected a refusal for: {item}"),
        Err(message) => message,
    }
}

#[test]
fn the_wrapper_exports_the_mangled_e32main_and_calls_main() {
    let out = wrapper("", "fn main () -> Result < () > { Ok (()) }");
    assert!(
        out.contains(&format!("export_name = \"{E32MAIN}\"")),
        "{out}"
    );
    assert!(
        out.contains("extern \"C\" fn __symbian_e32main () -> i32")
            || out.contains("extern \"C\" fn __symbian_e32main() -> i32"),
        "{out}"
    );
    assert!(out.contains("::symbian_std::__start(main)"), "{out}");
}

#[test]
fn the_wrapper_is_the_same_whatever_main_returns() {
    let unit = wrapper("", "fn main () { }");
    assert_eq!(unit, wrapper("", "fn main () -> i32 { 0 }"));
    assert_eq!(
        unit,
        wrapper("", "fn main () -> io :: Result < i32 > { Ok (0) }")
    );
}

#[test]
fn attributes_docs_and_a_pub_const_main_are_all_read_through() {
    let item = "#[doc = \" a note with ] and \\\" in it\"] #[inline] pub const fn main () { }";
    assert!(wrapper("", item).contains("__start(main)"));
}

#[test]
fn a_body_that_mentions_fn_or_a_brace_does_not_confuse_the_reader() {
    let item = "fn main () -> Result < () > { let s = \") fn main (\" ; Ok (()) }";
    assert!(wrapper("", item).contains("__start(main)"));
}

/// A token stream renders as the source it came from, so the macro sees the comments
/// and the doc comments as the user typed them, not as `#[doc = "…"]`.
#[test]
fn doc_comments_and_comments_are_read_through() {
    let item = "/// What the program does.\n// a note\n#[inline]\n/* and a block /* nested */ */\nfn main(/* nothing */) -> Result<()> {\n    Ok(())\n}";
    assert!(wrapper("", item).contains("__start(main)"));
}

#[test]
fn a_doc_comment_naming_a_function_is_not_mistaken_for_one() {
    let item = "/// Not `fn run()`, whatever this says.\nstruct Main;";
    assert!(refusal("", item).contains("`struct`"));
}

#[test]
fn something_that_is_not_a_function_says_so() {
    let message = refusal("", "struct Main ;");
    assert!(
        message.contains("can only be applied to a function") && message.contains("`struct`"),
        "{message}"
    );
}

#[test]
fn arguments_are_refused_and_the_message_shows_them() {
    let message = refusal("", "fn main (argc : i32) -> i32 { argc }");
    assert!(
        message.contains("must take no arguments") && message.contains("argc : i32"),
        "{message}"
    );
}

#[test]
fn a_name_other_than_main_is_refused_and_named() {
    let message = refusal("", "fn run () { }");
    assert!(
        message.contains("must be called `main`") && message.contains("`run`"),
        "{message}"
    );
}

#[test]
fn a_generic_main_is_refused() {
    let message = refusal("", "fn main < T > () { }");
    assert!(message.contains("cannot be generic"), "{message}");
}

#[test]
fn an_async_main_points_at_the_step_that_will_allow_it() {
    let message = refusal("", "async fn main () { }");
    assert!(
        message.contains("`async fn`") && message.contains("step 73"),
        "{message}"
    );
}

#[test]
fn an_unsafe_or_extern_main_is_refused() {
    assert!(refusal("", "unsafe fn main () { }").contains("`unsafe fn`"));
    assert!(refusal("", "extern \"C\" fn main () { }").contains("`extern fn`"));
}

#[test]
fn the_gui_shape_exports_the_app_functions_and_no_e32main() {
    let out = wrapper("gui", "fn main () -> Notes { Notes :: new () }");
    assert!(
        !out.contains(E32MAIN),
        "a GUI application's E32Main belongs to the shim: {out}"
    );
    // One call: the eight `symrs_app_*` exports live next to their bodies in
    // `symbian-ui`, and `create` is this `main`.
    assert_eq!(out, "::symbian_std::ui::__export_app!(Notes, main);\n");
}

/// The application type is read out of the signature, whatever shape it has, so that
/// it is never written a second time beside the one in `fn main`.
#[test]
fn the_gui_shape_reads_the_application_type_from_the_return_type() {
    for (item, app) in [
        ("fn main () -> app :: Notes { todo ! () }", "app :: Notes"),
        ("fn main () -> Notes < 4 > { todo ! () }", "Notes < 4 >"),
        (
            "fn main () -> Notes where Notes : Sized { todo ! () }",
            "Notes",
        ),
    ] {
        let out = wrapper("gui", item);
        assert!(
            out.contains(&format!("__export_app!({app}, main)")),
            "{item}: {out}"
        );
    }
}

#[test]
fn a_gui_main_that_returns_nothing_says_what_it_must_return() {
    let message = refusal("gui", "fn main () { }");
    assert!(
        message.contains("must return the application type") && message.contains("impl App"),
        "{message}"
    );
}

/// Everything the console shape refuses, the GUI shape refuses the same way: the
/// checks are on the signature, not on the shape.
#[test]
fn the_gui_shape_keeps_the_console_shapes_refusals() {
    assert!(refusal("gui", "async fn main () -> Notes { todo ! () }").contains("`async fn`"));
    assert!(refusal("gui", "fn run () -> Notes { todo ! () }").contains("must be called `main`"));
    assert!(
        refusal("gui", "fn main (a : i32) -> Notes { todo ! () }")
            .contains("must take no arguments")
    );
}

#[test]
fn an_unknown_argument_lists_what_there_is() {
    let message = refusal("console", "fn main () { }");
    assert!(
        message.contains("takes no arguments") && message.contains("`gui`"),
        "{message}"
    );
}
