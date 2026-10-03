use std::path::{Path, PathBuf};

use super::rust_build::{gui, rust};
use super::*;
use crate::sdk_lld_copy::SdkLldCopy;

fn lld_line(uid3_symbol: Option<u32>) -> LldLine {
    LldLine {
        sdk_lib: PathBuf::from("/sdk/epoc32/release/armv5/lib"),
        sdk_urel: PathBuf::from("/sdk/epoc32/release/armv5/urel"),
        copy: SdkLldCopy::at(PathBuf::from("/cache/k")),
        script: PathBuf::from("/rs/targets/symbian-lld.ld"),
        uid3_symbol,
    }
}

const ARCHIVE: &str = "/p/build/cargo/arm-symbian-e32/release/libhello.a";
const LIBCALLS: &str = "/p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib";

/// A `[ui]` application with the prebuilt set: experiment 109 §5's no-GCCE line, argv for
/// argv but for the paths and the trailing `/` the recorded line puts after `gcc_lib`.
#[test]
fn a_gui_app_with_the_prebuilt_set_links_with_the_experiment_109_line() {
    let b = gui();
    let pre = PathBuf::from("/pre/lib");
    let linker = Linker::lld(Path::new("/rust/bin/rust-lld"), pre.clone(), pre.clone());
    let shims = [pre.join("libsymrs_ui.a"), pre.join("libsymrs.a")];
    let line = b.link_line(
        &linker,
        &[ARCHIVE.into()],
        &shims,
        Some(Path::new(LIBCALLS)),
        Path::new("/p/build/hello.first.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let got = lld_line(Some(b.gcce.uid3)).adapt(line.unwrap()).unwrap();
    let want = "/rust/bin/rust-lld -flavor gnu -L/pre/lib/ -L /pre/lib --target1-abs \
        --no-undefined -nostdlib -shared -Ttext 0x8000 -Tdata 0x400000 -soname \
        hello{000a0000}[e79e4cf9].exe --target1-abs --no-undefined -nostdlib --strip-debug \
        --entry _E32Startup -u _E32Startup --gc-sections -u _Z7E32Mainv -u symrs_app_create \
        -L/cache/k/urel -l:eexe.lib -o /p/build/hello.first.elf -Map /p/build/hello.exe.map \
        -L/cache/k/lib -l:euser.dso -l:drtaeabi.dso \
        /p/build/cargo/arm-symbian-e32/release/libhello.a /pre/lib/libsymrs_ui.a \
        /pre/lib/libsymrs.a /p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib \
        -( -l:usrt2_2.lib -) -L/pre/lib -L/cache/k/lib -l:euser.dso -l:dfpaeabi.dso \
        -l:dfprvct2_2.dso -l:drtaeabi.dso -l:scppnwdl.dso -l:drtrvct2_2.dso -l:apparc.dso \
        -l:cone.dso -l:eikcore.dso -l:avkon.dso -l:gdi.dso --as-needed -l:efsrv.dso \
        -l:bafl.dso -l:esock.dso -l:insock.dso -l:eikcoctl.dso -l:eikctl.dso \
        --no-as-needed -lsupc++ -lgcc -z notext --target2=abs -Bsymbolic -T \
        /rs/targets/symbian-lld.ld --defsym=symrs_uid3=0xe79e4cf9";
    assert_eq!(got.join(" "), want);
}

/// A source checkout's console application: GCCE's runtime directories stay, the SDK's
/// are swapped for the fixed copies, and no UID symbol is defined (its shim, compiled
/// per application, has the UID on its compile line).
#[test]
fn a_checkout_build_keeps_the_gcce_runtime_and_defines_no_uid_symbol() {
    let b = rust();
    let g = b.gcce.tools.gcce().unwrap();
    let linker = Linker::lld(
        Path::new("/rust/bin/rust-lld"),
        g.gcc_lib.clone(),
        g.gcc_target_lib.clone(),
    );
    let shim = PathBuf::from("/p/build/shims/libsymrs.a");
    let (archive, elf, map) = (
        Path::new(ARCHIVE),
        Path::new("/p/build/hello.first.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let gnu = b
        .link_args(&[archive.into()], Some(&shim), None, elf, map)
        .unwrap();
    let got = lld_line(None)
        .adapt(
            b.link_line(&linker, &[archive.into()], &[shim], None, elf, map)
                .unwrap(),
        )
        .unwrap();
    // Against the GNU line of the same build: the program, one option fewer, the SDK's
    // directories, and the five rust-lld options at the end.
    let mut want: Vec<String> = s(&["/rust/bin/rust-lld", "-flavor", "gnu"]);
    want.extend(
        gnu[1..]
            .iter()
            .filter(|a| *a != "--default-symver")
            .map(|a| match a.as_str() {
                "-L/sdk/epoc32/release/armv5/lib" => "-L/cache/k/lib".to_string(),
                "-L/sdk/epoc32/release/armv5/urel" => "-L/cache/k/urel".to_string(),
                _ => a.clone(),
            }),
    );
    want.extend(s(&["-z", "notext", "--target2=abs", "-Bsymbolic", "-T"]));
    want.push("/rs/targets/symbian-lld.ld".into());
    assert_eq!(got, want);
    assert!(gnu.iter().any(|a| a == "--default-symver"));
    assert!(gnu.iter().any(|a| a == "-L/sdk/epoc32/release/armv5/lib"));
}

#[test]
fn the_sdk_files_are_the_lines_colon_libraries_each_once() {
    let line = s(&[
        "-l:eexe.lib",
        "-o",
        "x",
        "-l:euser.dso",
        "-(",
        "-l:usrt2_2.lib",
        "-)",
        "-l:euser.dso",
        "-lsupc++",
        "-lgcc",
        "-L/a",
    ]);
    assert_eq!(
        LldLine::sdk_files(&line),
        s(&["eexe.lib", "euser.dso", "usrt2_2.lib"])
    );
}

/// Experiment 112 §3: the second link is the first one writing the final ELF, with the
/// stubs object after every other input and a `--wrap` per stubbed function.
#[test]
fn the_second_link_appends_the_stubs_and_wraps_each_function() {
    let first = s(&[
        "/rust-lld",
        "-flavor",
        "gnu",
        "-o",
        "/b/h.first.elf",
        "a.a",
        "-T",
        "x.ld",
    ]);
    let got = LldLine::second_link(
        &first,
        Path::new("/b/h.elf"),
        Path::new("/b/import_stubs.o"),
        &s(&["_ZN4User4ExitEi", "_ZN4User9InfoPrintERK7TDesC16"]),
    )
    .unwrap();
    let want = s(&[
        "/rust-lld",
        "-flavor",
        "gnu",
        "-o",
        "/b/h.elf",
        "a.a",
        "-T",
        "x.ld",
        "/b/import_stubs.o",
        "--wrap=_ZN4User4ExitEi",
        "--wrap=_ZN4User9InfoPrintERK7TDesC16",
    ]);
    assert_eq!(got, want);
}

/// A line the rules cannot apply to is refused, not passed on half-changed: the second link
/// would otherwise overwrite the first ELF and a stale `<name>.elf` be post-linked.
#[test]
fn a_line_without_an_output_or_the_sdk_directories_is_refused() {
    let first = s(&["/rust-lld", "-flavor", "gnu", "a.a"]);
    let e = LldLine::second_link(&first, Path::new("/b/h.elf"), Path::new("/b/s.o"), &[]);
    assert!(e.unwrap_err().to_string().contains("no -o"));
    let line = s(&[
        "/rust-lld",
        "-L/sdk/epoc32/release/armv5/lib",
        "-l:euser.dso",
    ]);
    let e = lld_line(None).adapt(line).unwrap_err().to_string();
    assert!(e.contains("-L/sdk/epoc32/release/armv5/urel"), "{e}");
}
