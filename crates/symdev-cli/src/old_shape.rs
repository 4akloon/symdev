//! `OldShape`: a Rust project in 0.3.0's shape — a `staticlib` symdev linked — and the
//! edits that bring it to the shape cargo links through `symdev-ld` (design spec §8). 0.4.0
//! has one build path, so the old shape is refused rather than built the old way.

/// A 0.3.0 project's files that still need an edit, each with what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OldShape {
    edits: Vec<String>,
}

impl OldShape {
    /// `Some` when `Cargo.toml` still builds a `staticlib`; the edits list only what each
    /// file still needs.
    pub fn detect(cargo_toml: &str, main_rs: &str, config: &str, package: &str) -> Option<Self> {
        if !cargo_toml.contains("\"staticlib\"") {
            return None;
        }
        let mut edits = vec![format!(
            "Cargo.toml: remove `autobins = false` and the `[lib]` table, and add\n\
             [[bin]]\nname = \"{package}\"\npath = \"src/main.rs\"\ntest = false"
        )];
        if !main_rs.contains("#![no_main]") {
            edits.push(
                "src/main.rs: add `#![no_main]` on the line after `#![no_std]` (the \
                 `#[symbian_std::main]` attribute keeps `fn main`)"
                    .into(),
            );
        }
        let mut config_lines = Vec::new();
        if !config.contains("panic-abort-tests") {
            config_lines.push("under [unstable]: panic-abort-tests = true");
        }
        if !config.contains("linker = \"symdev-ld\"")
            || !config.contains("runner = \"symdev run --exe\"")
        {
            config_lines.push(
                "at the end:\n[target.arm-symbian-e32]\nlinker = \"symdev-ld\"\nrunner = \"symdev run --exe\"",
            );
        }
        if !config_lines.is_empty() {
            edits.push(format!(
                ".cargo/config.toml: add {}",
                config_lines.join("; and ")
            ));
        }
        Some(Self { edits })
    }

    /// The refusal: what changed, then the numbered edits.
    pub fn message(&self) -> String {
        let mut out = String::from(
            "this project has 0.3.0's shape (a staticlib that symdev linked); symdev 0.4.0 \
             builds it with cargo. Make these edits, then run symdev build again:",
        );
        for (i, edit) in self.edits.iter().enumerate() {
            out.push_str(&format!("\n{}. {edit}", i + 1));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_0_3_0_project_gets_every_edit_it_needs() {
        let cargo = "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
            # `src/main.rs` is a library: rustc never links, symdev does.\nautobins = false\n\n\
            [lib]\npath = \"src/main.rs\"\ncrate-type = [\"staticlib\"]\n\n[dependencies]\n";
        let config = "[build]\ntarget = \"build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json\"\n\
            target-dir = \"build/cargo\"\n\n[unstable]\nbuild-std = [\"core\", \"alloc\"]\n\
            build-std-features = [\"optimize_for_size\"]\njson-target-spec = true\n";
        let main = "#![no_std]\n\nuse symbian_core::Result;\n";
        let m = super::OldShape::detect(cargo, main, config, "hello")
            .unwrap()
            .message();
        for want in [
            "Cargo.toml",
            "remove `autobins = false` and the `[lib]` table",
            "[[bin]]\nname = \"hello\"\npath = \"src/main.rs\"\ntest = false",
            "src/main.rs",
            "#![no_main]",
            ".cargo/config.toml",
            "panic-abort-tests = true",
            "linker = \"symdev-ld\"",
            "runner = \"symdev run --exe\"",
        ] {
            assert!(m.contains(want), "{want}\n{m}");
        }
    }

    #[test]
    fn a_project_in_the_new_shape_is_left_alone() {
        let cargo = "[package]\nname = \"hello\"\n\n[[bin]]\nname = \"hello\"\npath = \"src/main.rs\"\ntest = false\n";
        assert!(super::OldShape::detect(cargo, "#![no_std]\n#![no_main]\n", "", "hello").is_none());
    }
}
