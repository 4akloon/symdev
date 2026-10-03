//! `#[symbian_test::tests]`: a module of `#[test] fn`s becomes a program (design spec §7).
//!
//! It works on the item's text, like [`crate::entry`], so its unit tests run outside a
//! macro expansion. A small scanner skips string and char literals and accepts any spacing
//! rustc prints between `#`, `[`, `test` and `]`.
pub struct TestModule;

const SHAPE: &str = "`#[symbian_test::tests]` goes on an inline `mod <name> { … }`";

impl TestModule {
    /// The expanded source for `item` (the module's text), or the message for
    /// `compile_error!`.
    pub fn expand(item: &str) -> Result<String, String> {
        let rest = item.trim_start();
        let after_mod = rest
            .strip_prefix("pub ")
            .unwrap_or(rest)
            .trim_start()
            .strip_prefix("mod")
            .ok_or_else(|| SHAPE.to_string())?;
        let name: String = after_mod
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let open = item.find('{').ok_or_else(|| SHAPE.to_string())?;
        let close = item
            .rfind('}')
            .filter(|c| *c > open)
            .ok_or_else(|| SHAPE.to_string())?;
        if name.is_empty() || item[..open].contains(';') {
            return Err(SHAPE.into());
        }
        let (body, tests) = Self::strip_tests(&item[open + 1..close]);
        if tests.is_empty() {
            return Err(format!(
                "`mod {name}` has no `#[test]` fn for `#[symbian_test::tests]` to run"
            ));
        }
        let cases: Vec<String> = tests
            .iter()
            .map(|t| format!("::symbian_test::Case {{ name: \"{t}\", run: {t} }}"))
            .collect();
        Ok(format!(
            "{head}{{{body}\npub(super) const __SYMBIAN_TESTS: &[::symbian_test::Case] = &[{list}];\n}}\n\
             #[unsafe(export_name = \"_Z7E32Mainv\")]\n\
             pub extern \"C\" fn __symbian_e32main() -> i32 {{\n\
             ::symbian_std::__start(|| ::symbian_test::__run(env!(\"CARGO_CRATE_NAME\"), \
             ::symbian_std::uid3!(), {name}::__SYMBIAN_TESTS))\n}}\n",
            head = &item[..open],
            list = cases.join(", ")
        ))
    }

    /// The body without its `#[test]` attributes, and the fns that carried one, in order.
    fn strip_tests(body: &str) -> (String, Vec<String>) {
        let (mut out, mut names) = (String::new(), Vec::new());
        let mut i = 0;
        while i < body.len() {
            let rest = &body[i..];
            let ch = rest.chars().next().unwrap_or(' ');
            match ch {
                '"' => {
                    let end = i + Self::literal_len(rest);
                    out.push_str(&body[i..end]);
                    i = end;
                }
                '\'' if Self::is_char_literal(rest) => {
                    let end = i + Self::literal_len(rest);
                    out.push_str(&body[i..end]);
                    i = end;
                }
                '#' => match Self::test_attribute_end(body, i) {
                    Some(end) => {
                        if let Some(name) = Self::next_fn_name(&body[end..]) {
                            names.push(name);
                        }
                        i = end;
                    }
                    None => {
                        out.push('#');
                        i += 1;
                    }
                },
                _ => {
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        (out, names)
    }

    /// `'x'`, `'é'` or `'\n'`, `'\u{…}'`; not the `'a` of a lifetime.
    fn is_char_literal(text: &str) -> bool {
        let mut chars = text.chars().skip(1);
        match chars.next() {
            Some('\\') => true,
            Some(_) => chars.next() == Some('\''),
            None => false,
        }
    }

    /// `#` `[` `test` `]` with any whitespace between: the index after `]`.
    fn test_attribute_end(body: &str, at: usize) -> Option<usize> {
        let rest = body[at + 1..]
            .trim_start()
            .strip_prefix('[')?
            .trim_start()
            .strip_prefix("test")?;
        let rest = rest.trim_start().strip_prefix(']')?;
        Some(body.len() - rest.len())
    }

    fn next_fn_name(text: &str) -> Option<String> {
        let at = text.find("fn ")?;
        let name: String = text[at + 3..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!name.is_empty()).then_some(name)
    }

    /// The length of the string or char literal `text` starts with, quotes included.
    fn literal_len(text: &str) -> usize {
        let b = text.as_bytes();
        let quote = b[0];
        let mut i = 1;
        while i < b.len() && b[i] != quote {
            i += if b[i] == b'\\' { 2 } else { 1 };
        }
        (i + 1).min(b.len())
    }
}

#[cfg(test)]
mod tests {
    use super::TestModule;

    fn expand(src: &str) -> Result<String, String> {
        TestModule::expand(src)
    }

    #[test]
    fn test_attributes_are_stripped_and_listed_in_order() {
        let out = expand(
            "mod checks { #[test] fn first() -> R { Ok(()) } fn helper() {} \
                          #[test] fn second() -> R { Ok(()) } }",
        )
        .unwrap();
        assert!(!out.contains("#[test]"), "{out}");
        let (a, b) = (
            out.find("\"first\"").unwrap(),
            out.find("\"second\"").unwrap(),
        );
        assert!(a < b && !out.contains("\"helper\""), "{out}");
        assert!(
            out.contains("__SYMBIAN_TESTS") && out.contains("_Z7E32Mainv"),
            "{out}"
        );
        assert!(out.contains("checks::__SYMBIAN_TESTS"), "{out}");
    }

    #[test]
    fn a_module_with_no_test_is_an_error() {
        assert!(
            expand("mod m { fn f() {} }")
                .unwrap_err()
                .contains("no `#[test]` fn")
        );
    }

    #[test]
    fn a_lifetime_is_not_a_char_literal() {
        let out = expand(
            "mod m { fn f<'a>(x: &'a str) -> &'a str { x } #[test] fn t() -> R { Ok(()) } }",
        )
        .unwrap();
        assert!(!out.contains("#[test]") && out.contains("\"t\""), "{out}");
    }

    #[test]
    fn a_test_attribute_inside_a_string_is_left_alone() {
        let out = expand("mod m { const S: &str = \"#[test]\"; #[test] fn t() -> R { Ok(()) } }")
            .unwrap();
        assert!(
            out.contains("\"#[test]\";") && out.contains("Case { name: \"t\""),
            "{out}"
        );
    }

    #[test]
    fn anything_but_an_inline_module_is_an_error() {
        assert!(
            expand("fn f() {}")
                .unwrap_err()
                .contains("`mod <name> { … }`")
        );
        assert!(expand("mod m;").unwrap_err().contains("`mod <name> { … }`"));
    }
}
