//! `symdev new <name> --language rust`: a Cargo project on the Rust SDK (experiment 65).
//! No `bld.inf`, no `.mmp`: `RustBuild` runs cargo and then the recorded link line.
use std::path::{Path, PathBuf};

use symdev_build::{RustSdk, RustSdkLink};
use symdev_core::Error;

use crate::build_dir::BuildDir;
use crate::cli::Template;
use crate::scaffold::{io_err, uid3_hex};

pub fn write_rust(
    root: &Path,
    name: &str,
    template: Template,
    sdk: impl FnOnce() -> Result<RustSdk, Error>,
) -> Result<PathBuf, Error> {
    if template != Template::Console {
        return Err(Error::Other(
            "TODO: --language rust supports only --template console (the GUI template \
             needs the Avkon framework, which the Rust SDK does not reach yet)"
                .into(),
        ));
    }
    let sdk = sdk()?;
    // The resolved SDK's own file, not the one this symdev was built with: the build
    // refuses a project whose nightly is not its SDK's.
    let toolchain = sdk.toolchain()?;
    let toolchain = std::fs::read_to_string(toolchain.path()).map_err(io_err)?;
    std::fs::create_dir_all(root.join("src")).map_err(io_err)?;
    std::fs::create_dir_all(root.join(".cargo")).map_err(io_err)?;
    std::fs::create_dir_all(root.join("tests")).map_err(io_err)?;
    // `build/` ignores itself before the link is made in it, as `symdev build` does.
    BuildDir::of(root).create()?;
    RustSdkLink::of(root).point_at(&sdk)?;
    let files = [
        ("symdev.toml".to_string(), manifest(name)),
        ("Cargo.toml".into(), cargo_manifest(name)),
        (".cargo/config.toml".into(), cargo_config()),
        ("rust-toolchain.toml".into(), toolchain),
        ("src/main.rs".into(), RustSdk::HELLO_MAIN.into()),
        ("tests/smoke.rs".into(), SMOKE_TEST.into()),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).map_err(io_err)?;
    }
    Ok(root.to_path_buf())
}

/// The scaffold's test: `cargo test` builds it, the runner installs and runs it.
const SMOKE_TEST: &str = "\
//! A test on the device: `cargo test` builds it, symdev installs and runs it, and prints
//! what it reports (symbian-test).
#![no_std]
#![no_main]

#[symbian_test::tests]
mod smoke {
    use symbian_test::{Evidence, ensure};

    #[test]
    fn arithmetic() -> Result<(), Evidence> {
        ensure(2 + 2 == 4, \"2 + 2 is 4\")
    }
}
";

fn manifest(name: &str) -> String {
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         \n\
         [target]\n\
         device = \"nokia-e52\"\n\
         \n\
         [language]\n\
         name = \"rust\"\n\
         \n\
         [symbian]\n\
         uid3 = \"{}\"\n\
         capabilities = []\n\
         vendor = \"symdev\"\n\
         \n\
         [signing]\n\
         mode = \"self-signed\"\n",
        uid3_hex(name)
    )
}

/// A binary named after the package, which cargo links through `symdev-ld`, and a
/// `harness = false` test on `symbian-test`; the SDK crates through `build/rust-sdk`
/// ([`RustSdkLink`]); the same profile as the SDK workspace (size, one object, no
/// unwinder).
///
/// `symbian-std` and not `symbian-runtime`: the entry point is `#[symbian_std::main]`
/// (experiment 81), and the runtime underneath it — the panic handler, the heap and
/// the older `entry!` — comes with it, so the template names one crate for the road
/// and one (`symbian-core`) for the descriptors the escape hatch still needs.
fn cargo_manifest(name: &str) -> String {
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         # No libtest on the phone: tests are tests/*.rs with harness = false (symbian-test).\n\
         [[bin]]\n\
         name = \"{name}\"\n\
         path = \"src/main.rs\"\n\
         test = false\n\
         \n\
         [[test]]\n\
         name = \"smoke\"\n\
         harness = false\n\
         \n\
         [dependencies]\n\
         symbian-core = {{ path = \"{}\" }}\n\
         symbian-std = {{ path = \"{}\" }}\n\
         \n\
         [dev-dependencies]\n\
         symbian-test = {{ path = \"{}\" }}\n\
         \n\
         [profile.release]\n\
         opt-level = \"s\"\n\
         lto = true\n\
         codegen-units = 1\n\
         panic = \"abort\"\n\
         debug = false\n\
         \n\
         # `compiler_builtins` comes out of `-Zbuild-std` as one object under the\n\
         # workspace's `codegen-units = 1`, so one reference pulls all of it: a single\n\
         # integer division cost `examples/time` 9 168 bytes of soft-float it never\n\
         # calls. One codegen unit per builtin, and the linker takes only the member\n\
         # it needs.\n\
         [profile.release.package.compiler_builtins]\n\
         codegen-units = 10000\n\
         \n\
         [profile.dev]\n\
         panic = \"abort\"\n",
        RustSdkLink::crate_dir("symbian-core"),
        RustSdkLink::crate_dir("symbian-std"),
        RustSdkLink::crate_dir("symbian-test")
    )
}

/// What `symdev build` passes on the cargo command line, so a hand `cargo build` (or
/// `cargo clippy`) in the project does the same once `build/rust-sdk` exists: a fresh
/// clone, or one after `rm -rf build`, has no link until a `symdev build` makes it. The
/// target spec is relative to the project, wherever cargo is run from in it (experiment
/// 110 c).
fn cargo_config() -> String {
    format!(
        "# Kept in step with `symdev build` (RustBuild::cargo_args). The Rust SDK is reached\n\
         # through build/rust-sdk, a link `symdev build` makes: in a fresh clone or after\n\
         # `rm -rf build`, run `symdev build` once before cargo or rust-analyzer.\n\
         [build]\n\
         target = \"{}\"\n\
         target-dir = \"build/cargo\"\n\
         \n\
         [unstable]\n\
         build-std = [\"core\", \"alloc\"]\n\
         # `core`'s size/speed switch: the small integer `Display` (no 200-byte\n\
         # two-digit table), the small sort, the short padding path.\n\
         build-std-features = [\"optimize_for_size\"]\n\
         json-target-spec = true\n\
         # cargo test builds core twice without it (E0152, experiment 114 §1.1).\n\
         panic-abort-tests = true\n\
         \n\
         # cargo links through symdev (signed .sisx beside the image) and runs on a device.\n\
         [target.arm-symbian-e32]\n\
         linker = \"symdev-ld\"\n\
         runner = \"symdev run --exe\"\n",
        RustSdkLink::target_spec()
    )
}

#[cfg(test)]
mod tests;
