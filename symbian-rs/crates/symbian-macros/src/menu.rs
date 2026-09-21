//! `Menu`: the `[[ui.menu]]` items of `symdev.toml`, as the constants an application
//! matches on.
//!
//! A menu item's number lives in two artefacts made by two compilers — the `.rss`
//! symdev generates and compiles, and the Rust `match` — and the two must agree. The
//! C++ SDK made that one definition by `#include`-ing the same `.hrh` from both; this
//! is the same move for Rust: the attribute reads the same `symdev.toml` symdev reads,
//! with the same reader, and writes one `Command` constant per item into a `menu`
//! module. A word that is not in the manifest is then not a constant, and a mistyped,
//! renamed or deleted item is `E0425` with the nearest name suggested — not a menu
//! entry that silently never fires (`docs/research/command-id-design.md`).
use std::path::{Path, PathBuf};

use crate::entry::UI;

/// The file next to `Cargo.toml` the menu is read from.
pub const MANIFEST: &str = "symdev.toml";

/// The menu of one application: the constant name and the manifest word of each item.
pub struct Menu {
    items: Vec<(String, String)>,
    /// The manifest this came from, named in the generated code so that cargo rebuilds
    /// the crate when it changes.
    path: Option<PathBuf>,
}

impl Menu {
    /// Reads `<dir>/symdev.toml` with symdev's own manifest reader.
    ///
    /// The error is the manifest's own (a duplicate id, two ids that hash alike, a
    /// missing `[ui]`), so the application sees the same refusal at `cargo build` that
    /// `symdev build` would give it.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join(MANIFEST);
        let manifest = symdev_manifest::load(&path).map_err(|e| {
            format!(
                "`#[{}(gui)]` cannot read {}: {e}",
                crate::entry::ATTRIBUTE,
                path.display()
            )
        })?;
        let Some(ui) = manifest.ui else {
            return Err(format!(
                "`#[{}(gui)]` is an Avkon application, but {} has no `[ui]` section",
                crate::entry::ATTRIBUTE,
                path.display()
            ));
        };
        let items = ui
            .menu
            .iter()
            .map(|item| (item.constant(), item.name.clone()))
            .collect();
        Ok(Self {
            items,
            path: Some(path),
        })
    }

    /// A menu from names alone, for the tests: no file is involved.
    #[cfg(test)]
    pub fn of(items: &[(&str, &str)]) -> Self {
        Self {
            items: items
                .iter()
                .map(|(constant, name)| (constant.to_string(), name.to_string()))
                .collect(),
            path: None,
        }
    }

    /// `mod menu { pub const MORE: Command = Command::named("more"); … }`.
    ///
    /// Each constant is `Command::named` of the manifest word, so the number is still
    /// derived where it always was; what is new is that the *name* has to exist. The
    /// `include_str!` is what makes cargo recompile the crate when the manifest
    /// changes: a proc macro has no `rerun-if-changed`, but a file named by an
    /// `include_str!` in its output lands in rustc's dep-info, and that is what cargo
    /// consults (measured in `docs/research/command-id-design.md`). The constant is
    /// never used, so nothing of the file reaches the binary.
    pub fn module(&self) -> String {
        let mut out = String::new();
        if let Some(path) = &self.path {
            out.push_str(&format!(
                "const _: &str = ::core::include_str!(\"{}\");\n",
                escape(&path.display().to_string())
            ));
        }
        out.push_str(
            "/// The Options menu, one constant per `[[ui.menu]]` item of `symdev.toml`.\n\
             #[allow(dead_code)]\n\
             mod menu {\n",
        );
        for (constant, name) in &self.items {
            out.push_str(&format!(
                "    /// `[[ui.menu]] id = \"{name}\"`.\n    \
                 pub const {constant}: {UI}::Command = {UI}::Command::named(\"{}\");\n",
                escape(name)
            ));
        }
        out.push_str("}\n");
        out
    }
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::Menu;

    #[test]
    fn one_constant_per_item_named_by_the_manifest_word() {
        let out = Menu::of(&[("MORE", "more"), ("NEW_NOTE", "new-note")]).module();
        assert!(out.contains("mod menu {"), "{out}");
        assert!(
            out.contains("pub const MORE: ::symbian_std::ui::Command = ::symbian_std::ui::Command::named(\"more\");"),
            "{out}"
        );
        assert!(
            out.contains("pub const NEW_NOTE: ::symbian_std::ui::Command = ::symbian_std::ui::Command::named(\"new-note\");"),
            "{out}"
        );
    }

    #[test]
    fn an_empty_menu_is_an_empty_module() {
        let out = Menu::of(&[]).module();
        assert!(out.contains("mod menu {\n}"), "{out}");
        assert!(!out.contains("include_str"), "{out}");
    }

    #[test]
    fn a_loaded_menu_names_its_file_so_cargo_tracks_it() {
        let dir = std::env::temp_dir().join(format!("symbian-macros-menu-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(super::MANIFEST),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n[target]\ndevice = \"nokia-e52\"\n\
             [language]\nname = \"rust\"\n[symbian]\nuid3 = \"0xe0000001\"\n[ui]\nkind = \"avkon\"\n\
             [[ui.menu]]\nid = \"more\"\nlabel = \"More\"\n",
        )
        .unwrap();
        let out = Menu::load(&dir)
            .map_err(|e| e.to_string())
            .unwrap()
            .module();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(out.contains("include_str!("), "{out}");
        assert!(out.contains("symdev.toml\");"), "{out}");
        assert!(out.contains("pub const MORE:"), "{out}");
    }

    #[test]
    fn a_gui_application_without_ui_is_refused_by_name() {
        let dir = std::env::temp_dir().join(format!("symbian-macros-noui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(super::MANIFEST),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n[target]\ndevice = \"nokia-e52\"\n\
             [language]\nname = \"rust\"\n[symbian]\nuid3 = \"0xe0000001\"\n",
        )
        .unwrap();
        let err = Menu::load(&dir).map(|_| ()).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("no `[ui]` section"), "{err}");
    }

    #[test]
    fn a_missing_manifest_names_the_path() {
        let err = Menu::load(std::path::Path::new("/nonexistent/dir"))
            .map(|_| ())
            .unwrap_err();
        assert!(err.contains("/nonexistent/dir/symdev.toml"), "{err}");
    }
}
