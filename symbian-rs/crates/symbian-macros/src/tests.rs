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
    assert!(
        out.contains("::symbian_std::__rt::ExitCode::from_main(main())"),
        "{out}"
    );
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
    assert!(wrapper("", item).contains("from_main(main())"));
}

#[test]
fn a_body_that_mentions_fn_or_a_brace_does_not_confuse_the_reader() {
    let item = "fn main () -> Result < () > { let s = \") fn main (\" ; Ok (()) }";
    assert!(wrapper("", item).contains("from_main(main())"));
}

/// A token stream renders as the source it came from, so the macro sees the comments
/// and the doc comments as the user typed them, not as `#[doc = "…"]`.
#[test]
fn doc_comments_and_comments_are_read_through() {
    let item = "/// What the program does.\n// a note\n#[inline]\n/* and a block /* nested */ */\nfn main(/* nothing */) -> Result<()> {\n    Ok(())\n}";
    assert!(wrapper("", item).contains("from_main(main())"));
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
fn the_gui_shape_is_grammar_today_and_code_in_step_75() {
    let message = refusal("gui", "fn main () { }");
    assert!(
        message.contains("not implemented yet")
            && message.contains("CActiveScheduler")
            && message.contains("step 75"),
        "{message}"
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
