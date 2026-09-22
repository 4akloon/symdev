//! `strings!()`: one constant per key of the project's `locales/`.
//!
//! The constants are read with `symdev-locale` — the same reader, the same completeness
//! check and the same index `symdev build` compiles the resource files with — so a key
//! the source names is a key the compiled file has, at the index the source says. A key
//! that is not in `locales/default.toml` is not a constant, which makes a misspelt or
//! removed string `E0425` at the line that uses it; a translation missing from another
//! language is a `compile_error!` here, and `symdev build` refuses it too.
//!
//! Every file read is named in an `include_str!` whose value is discarded, so cargo
//! rebuilds the crate when one of them changes (the rebuild-tracking finding of the
//! command-id research: a proc macro that reads a file is otherwise not re-run). A
//! language file *added* later is not tracked by this crate — it changes no constant —
//! and `symdev build` checks it when it compiles the resources.
use std::fmt::Write as _;
use std::path::Path;

use symdev_locale::Locales;

/// The path the generated constants name their type by.
const STR: &str = "::symbian_std::locale::Str";

/// The `pub mod strings { … }` for the project at `manifest_dir`, or the message to
/// report as a compile error.
pub fn expand(manifest_dir: &Path) -> Result<String, String> {
    let dir = manifest_dir.join("locales");
    let locales = Locales::load(&dir)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "`symbian_std::strings!()` reads {}, which does not exist: create \
                 locales/default.toml with one `key = \"text\"` per line",
                dir.display()
            )
        })?;
    let mut out = String::from("pub mod strings {\n");
    let mut files: Vec<String> = vec!["default.toml".into()];
    files.extend(
        locales
            .variants
            .iter()
            .map(|(l, _)| format!("{}.toml", l.name)),
    );
    for file in files {
        let _ = writeln!(
            out,
            "    const _: &str = ::core::include_str!({:?});",
            dir.join(file).display().to_string()
        );
    }
    for key in locales.keys() {
        let Some(index) = locales.index(key) else {
            continue;
        };
        let text = locales
            .default
            .entries
            .get(key)
            .cloned()
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "    #[doc = {text:?}]\n    pub const {}: {STR} = {STR}::at({index});",
            key.to_ascii_uppercase()
        );
    }
    out.push_str("}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::expand;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("locales")).unwrap();
        for (name, text) in files {
            std::fs::write(dir.path().join("locales").join(name), text).unwrap();
        }
        dir
    }

    #[test]
    fn one_constant_per_key_at_the_index_the_build_compiles_it_at() {
        let p = project(&[
            ("default.toml", "ok = \"ok\"\ngreeting = \"Hello\"\n"),
            ("french.toml", "ok = \"d'accord\"\ngreeting = \"Bonjour\"\n"),
        ]);
        let out = expand(p.path()).unwrap();
        assert!(out.contains("pub const GREETING: ::symbian_std::locale::Str = ::symbian_std::locale::Str::at(2);"), "{out}");
        assert!(
            out.contains(
                "pub const OK: ::symbian_std::locale::Str = ::symbian_std::locale::Str::at(3);"
            ),
            "{out}"
        );
        assert!(out.contains("#[doc = \"Hello\"]"), "{out}");
        assert!(
            out.contains("default.toml\");") && out.contains("french.toml\");"),
            "{out}"
        );
    }

    #[test]
    fn a_missing_translation_is_the_error_the_build_would_give() {
        let p = project(&[
            ("default.toml", "a = \"A\"\nb = \"B\"\n"),
            ("french.toml", "a = \"a\"\n"),
        ]);
        let e = expand(p.path()).unwrap_err();
        assert!(e.contains("french.toml") && e.contains("`b`"), "{e}");
    }

    #[test]
    fn no_locales_directory_says_what_to_create() {
        let dir = tempfile::tempdir().unwrap();
        assert!(expand(dir.path()).unwrap_err().contains("default.toml"));
    }
}
