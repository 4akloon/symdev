//! `RustSdk`: where the Rust SDK for the phone (`symbian-rs/`) lives on this host.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// The `symbian-rs/` workspace: the target JSON, the SDK crates a project depends on by
/// path, and the pinned toolchain. Found through `SYMDEV_RUST_SDK`, else the checkout
/// this `symdev` was built from (a stopgap until an installed SDK layout exists; the
/// scaffold writes the absolute paths into the project, experiment 65).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSdk {
    root: PathBuf,
}

impl RustSdk {
    /// The one Rust target (design spec §3): `targets/arm-symbian-e32.json`.
    pub const TARGET: &'static str = "arm-symbian-e32";
    /// `symbian-rs/rust-toolchain.toml`, copied verbatim into every scaffolded project so
    /// the project builds with the same pinned nightly as the SDK.
    pub const TOOLCHAIN_FILE: &'static str =
        include_str!("../../../symbian-rs/rust-toolchain.toml");
    /// The hello application (`symbian-rs/examples/hello`), the scaffold's `src/main.rs`:
    /// `#![no_std]`, `#[symbian_std::main]` and a `fn main` returning a `Result`
    /// (experiment 81). No `#![no_main]`: the crate is a `staticlib`, so rustc never
    /// looks for a `main` of its own.
    pub const HELLO_MAIN: &'static str =
        include_str!("../../../symbian-rs/examples/hello/src/main.rs");
    /// The import libraries the SDK's own crates and C++ shim need beyond the runtime
    /// set of the recorded link line: `efsrv.dso` for `RFs`, `bafl.dso` for the trapped
    /// `BaflUtils::EnsurePathExistsL`, `esock.dso` for `RSocketServ`, `RSocket` and
    /// `RHostResolver`, `insock.dso` for `TInetAddr` (step 74).
    ///
    /// They are named for every Rust application, because the SDK and not the project
    /// decides what the shim calls (design spec §7, step 70). An unused one costs
    /// nothing: the post-linker emits an import only for a symbol that is actually
    /// referenced, which is why the recorded C++ line has carried four unused DSOs since
    /// experiment 5 and still produces a 746-byte hello.
    pub const LIBRARIES: &'static [&'static str] =
        &["efsrv.dso", "bafl.dso", "esock.dso", "insock.dso"];

    /// What the Avkon shim of `shims/s60` imports, and **only** a project with a
    /// `[ui]` section gets them. The list is the six a minimal Avkon application
    /// needs, counted per `NEEDED` DSO on `examples/gui`'s ELF (experiment 76): cone
    /// 67 imports, eikcore 58, avkon 38, apparc 6, euser 11, gdi 1. `euser.dso` is
    /// already on the recorded link line, so five are named here.
    ///
    /// `ws32.dso` is deliberately absent: every drawing entry point is a pure virtual
    /// of `CGraphicsContext`, so painting through the gc the framework hands over
    /// costs no window-server import at all. `gdi.dso` is here for the one symbol
    /// `CFont::AscentInPixels` — which this shim does not yet call, so `--as-needed`
    /// would drop it; it stays named because the moment text is measured it is back.
    pub const UI_LIBRARIES: &'static [&'static str] = &[
        "apparc.dso",
        "cone.dso",
        "eikcore.dso",
        "avkon.dso",
        "gdi.dso",
    ];

    pub fn from_env() -> Result<Self> {
        let root = match std::env::var_os("SYMDEV_RUST_SDK") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../symbian-rs")),
        };
        Self::at(&root)
    }

    /// `root` must hold the target JSON; the path is canonicalised so the scaffold can
    /// write it into a project anywhere.
    pub fn at(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| {
            Error::Other(format!(
                "Rust SDK not found at {} ({e}); set SYMDEV_RUST_SDK to the symbian-rs directory",
                root.display()
            ))
        })?;
        let sdk = Self { root };
        if !sdk.target_spec().is_file() {
            return Err(Error::Other(format!(
                "Rust SDK at {} has no targets/{}.json; set SYMDEV_RUST_SDK to the symbian-rs directory",
                sdk.root.display(),
                Self::TARGET
            )));
        }
        Ok(sdk)
    }

    pub fn target_spec(&self) -> PathBuf {
        self.root
            .join("targets")
            .join(format!("{}.json", Self::TARGET))
    }

    /// The SDK crate that defines what rustc's own code generation calls and this
    /// platform does not provide: the `__atomic_*` family over an `RFastLock`,
    /// `__sync_synchronize`, `memcmp` and `bcmp`.
    ///
    /// It is **not** a dependency of the application. It is built separately and put on
    /// the link line as its own archive, so that a program which performs no atomic
    /// operation and compares no bytes carries none of it: a `#[unsafe(no_mangle)]`
    /// symbol is a global in a `-shared` link and therefore a `--gc-sections` root, so
    /// as an ordinary dependency it cost every program 756 bytes (measured on `hello`).
    pub const LIBCALLS_CRATE: &'static str = "symbian-libcalls";

    /// The cargo profile the libcall crate is built under. It exists only to turn LTO
    /// off: under the workspace's `lto = true` an rlib holds LLVM bitcode, which `ld`
    /// cannot read.
    pub const LIBCALLS_PROFILE: &'static str = "libcalls";

    /// `crates/symbian-libcalls/Cargo.toml`.
    pub fn libcalls_manifest(&self) -> PathBuf {
        self.crate_dir(Self::LIBCALLS_CRATE).join("Cargo.toml")
    }

    /// `crates/<name>` inside the SDK, for a path dependency.
    pub fn crate_dir(&self, name: &str) -> PathBuf {
        self.root.join("crates").join(name)
    }

    /// `shims/common`: the C++ the SDK compiles into every Rust application so that a
    /// leaving Symbian call is `TRAP`ped before it can reach a Rust frame (design spec
    /// §7, step 70).
    pub fn shim_dir(&self) -> PathBuf {
        self.root.join("shims").join("common")
    }

    /// `shims/s60`: the four Avkon subclasses, compiled **only** for a project whose
    /// manifest has a `[ui]` section. They bring five more import libraries and an
    /// `E32Main` of their own, neither of which a console application may be given.
    pub fn ui_shim_dir(&self) -> PathBuf {
        self.root.join("shims").join("s60")
    }

    /// Every `.cpp` of the shim, sorted, so the object list and therefore the link
    /// line are the same on every host. `ui` adds [`Self::ui_shim_dir`].
    ///
    /// The application names none of them: the shim is part of the SDK.
    pub fn shim_sources(&self, ui: bool) -> Result<Vec<PathBuf>> {
        let mut sources = self.sources_in(&self.shim_dir())?;
        if ui {
            sources.extend(self.sources_in(&self.ui_shim_dir())?);
        }
        Ok(sources)
    }

    fn sources_in(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut sources: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| {
                Error::Other(format!(
                    "Rust SDK at {} has no readable {} ({e}); the C++ shim is part of the SDK",
                    self.root.display(),
                    dir.display()
                ))
            })?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "cpp"))
            .collect();
        sources.sort();
        Ok(sources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_sdk_is_found_and_has_the_target() {
        let sdk = RustSdk::from_env().unwrap();
        assert!(sdk.target_spec().ends_with("targets/arm-symbian-e32.json"));
        assert!(sdk.crate_dir("symbian-std").join("Cargo.toml").is_file());
        assert!(RustSdk::TOOLCHAIN_FILE.contains("channel = \"nightly-"));
        assert!(RustSdk::HELLO_MAIN.contains("#[symbian_std::main]"));
        assert!(RustSdk::HELLO_MAIN.contains("fn main() -> Result<()>"));
        assert!(!RustSdk::HELLO_MAIN.contains("no_main"));
    }

    #[test]
    fn missing_sdk_names_the_env_var() {
        let err = RustSdk::at(Path::new("/nonexistent/symbian-rs")).unwrap_err();
        assert!(err.to_string().contains("SYMDEV_RUST_SDK"), "{err}");
    }
}
