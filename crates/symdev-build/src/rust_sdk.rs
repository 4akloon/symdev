//! `RustSdk`: where the Rust SDK for the phone (`symbian-rs/`) lives on this host.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};
use symdev_sdk::RustSdkWorkspace;

use crate::rust_prebuilt::RustPrebuilt;
use crate::rust_toolchain_file::RustToolchainFile;

/// The `symbian-rs/` workspace: the target JSON, the SDK crates a project depends on by
/// path, and the pinned toolchain. Which one a build uses is the CLI's `Provision`'s
/// business (spec §12): `SYMDEV_RUST_SDK`, else [`Self::CHECKOUT`] (none in a release
/// build), else the installed `rust-sdk` package. A project names it only through
/// `build/rust-sdk` ([`crate::RustSdkLink`], experiment 110), which every build points at
/// the one it resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSdk {
    root: PathBuf,
}

impl RustSdk {
    /// The `symbian-rs/` of the source checkout this `symdev` was built from. It is where
    /// [`Self::HELLO_MAIN`] was read; a developer working on the SDK keeps building against
    /// it while it exists.
    ///
    /// `None` in a release build: one compiled with `SYMDEV_RELEASE` set (to anything but
    /// the empty string), as the release recipe must do. A prebuilt binary's checkout
    /// would be a path of the machine that built it, where on another machine any local
    /// user could plant a `symbian-rs` for every other user's builds to pick up.
    pub const CHECKOUT: Option<&'static str> = Self::checkout(
        option_env!("SYMDEV_RELEASE"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../symbian-rs"),
    );
    /// `tree`, unless `release` (`SYMDEV_RELEASE` as the compiler saw it) is set and not
    /// empty.
    const fn checkout(release: Option<&str>, tree: &'static str) -> Option<&'static str> {
        match release {
            Some(release) if !release.is_empty() => None,
            _ => Some(tree),
        }
    }

    /// The one Rust target (design spec §3): `targets/arm-symbian-e32.json`.
    pub const TARGET: &'static str = "arm-symbian-e32";
    /// The hello application (`symbian-rs/examples/hello`), the scaffold's `src/main.rs`:
    /// `#![no_std]`, `#[symbian_std::main]` and a `fn main` returning a `Result`
    /// (experiment 81), and `#![no_main]`: the crate is a binary cargo links through
    /// `symdev-ld`, and the attribute keeps `fn main`, which rustc would otherwise take for
    /// the program's own entry (E0580, experiment 114 §1.1).
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
    pub const LIBRARIES: &'static [&'static str] = &[
        "efsrv.dso",
        "bafl.dso",
        "esock.dso",
        "insock.dso",
        "eikcoctl.dso",
        "eikctl.dso",
    ];

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
    ///
    /// The list box adds no name here: the two DSOs it needs beyond `avkon` are in
    /// [`Self::LIBRARIES`], which is the `--as-needed` group, so a GUI application
    /// without a list does not load them.
    pub const UI_LIBRARIES: &'static [&'static str] = &[
        "apparc.dso",
        "cone.dso",
        "eikcore.dso",
        "avkon.dso",
        "gdi.dso",
    ];

    /// What [`Self::at`] requires, relative to the SDK root: the target JSON, the
    /// `rust-toolchain.toml` a project's nightly is checked against, the SDK's workspace
    /// manifest (and, beyond this list, every member it names: [`RustSdkWorkspace`]), and
    /// what the tree reaches outside itself — `symbian-macros` reads locales files with the
    /// host crate `symdev-locale` (`../../../crates/symdev-locale`), which inherits its
    /// version and edition from the root `Cargo.toml`. A checkout has all of them; so does
    /// the `rust-sdk` package, which keeps the repository's layout.
    pub const REQUIRED: &'static [&'static str] = &[
        "targets/arm-symbian-e32.json",
        "rust-toolchain.toml",
        "Cargo.toml",
        "../crates/symdev-locale/Cargo.toml",
        "../Cargo.toml",
    ];

    /// `root` must hold [`Self::REQUIRED`]; the path is canonicalised so the scaffold can
    /// write it into a project anywhere. The error says what is wrong with `root`; which
    /// variable or package named it is for the caller to add.
    pub fn at(root: &Path) -> Result<Self> {
        let root = root
            .canonicalize()
            .map_err(|e| Error::Other(format!("Rust SDK not found at {} ({e})", root.display())))?;
        let no_sdk = |what: &str| {
            Error::Other(format!(
                "Rust SDK at {} has no {what}: it is the symbian-rs directory of a symdev \
                 checkout or of the rust-sdk package",
                root.display()
            ))
        };
        for file in Self::REQUIRED {
            if !root.join(file).is_file() {
                return Err(no_sdk(file));
            }
        }
        let workspace = RustSdkWorkspace::read(&root)?;
        if let Some(manifest) = workspace.missing_member(&root) {
            return Err(no_sdk(&format!("{manifest}, a member of its workspace")));
        }
        Ok(Self { root })
    }

    /// The `symbian-rs/` directory, canonical.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `rust-toolchain.toml`: the nightly the SDK is built with, which a project must name
    /// too.
    pub fn toolchain(&self) -> Result<RustToolchainFile> {
        match RustToolchainFile::read(&self.root)? {
            Some(file) if file.channel().is_some() => Ok(file),
            _ => Err(Error::Other(format!(
                "Rust SDK at {} has no {} that names a [toolchain] channel: it is the \
                 symbian-rs directory of a symdev checkout or of the rust-sdk package",
                self.root.display(),
                RustToolchainFile::NAME
            ))),
        }
    }

    pub fn target_spec(&self) -> PathBuf {
        self.root
            .join("targets")
            .join(format!("{}.json", Self::TARGET))
    }

    /// `targets/symbian-lld.ld`, the layout every rust-lld link takes (experiment 109 §2).
    /// Not in [`Self::REQUIRED`]: a GNU ld build does without it.
    pub fn lld_script(&self) -> Result<PathBuf> {
        let script = self.root.join("targets/symbian-lld.ld");
        match script.is_file() {
            true => Ok(script),
            false => Err(Error::Other(format!(
                "Rust SDK at {} has no targets/symbian-lld.ld, which rust-lld links with \
                 (it predates symdev's rust-lld link); set SYMDEV_RUST_LINKER=gnu to link \
                 with GCCE's GNU ld, or use a Rust SDK that has the script",
                self.root.display()
            ))),
        }
    }

    /// The shims and GCC runtime compiled once ([`RustPrebuilt`]), when the SDK has them.
    pub fn prebuilt(&self) -> Result<Option<RustPrebuilt>> {
        RustPrebuilt::in_sdk(&self.root)
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

    /// The SDK crate that holds the raw `extern "C"` declarations. It is copied into
    /// the patched `library/` when a `rust-std` project is built ([`crate::StdSrc`]),
    /// because `std`'s platform layer calls it and a crate outside the sysroot build
    /// graph gets no `core`.
    pub const SYS_CRATE: &'static str = "symbian-sys";

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

    /// `rust-src`: the patched-`std` overlay and the hashes of the toolchain files it
    /// replaces. See `symbian-rs/rust-src/README.md`.
    pub fn std_overlay_dir(&self) -> PathBuf {
        self.root.join("rust-src")
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
mod tests;
