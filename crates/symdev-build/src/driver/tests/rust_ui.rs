//! What a `[ui]` project adds to the link line and to the shim's compile line, and
//! what a console project must still not get.
use std::path::Path;

use super::rust_build::{gui, rust};
use super::*;
use crate::rust_sdk::RustSdk;

/// The three things `[ui]` adds to the link line, and the one thing it must not move.
#[test]
fn a_gui_link_names_the_vtable_and_the_avkon_libraries() {
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let console = rust().link_args(a, None, None, elf, map);
    let got = gui().link_args(a, None, None, elf, map);

    // `-u symrs_app_vtbl`, because the reference to it runs from the shim archive
    // back into the Rust archive, which ld has already passed.
    let u = |args: &[String], sym: &str| args.windows(2).any(|w| w[0] == "-u" && w[1] == sym);
    assert!(u(&got, crate::APP_VTBL), "{got:?}");
    assert!(u(&got, crate::E32MAIN), "{got:?}");
    assert!(!u(&console, crate::APP_VTBL), "a console app has no vtable");

    for lib in RustSdk::UI_LIBRARIES {
        let flag = format!("-l:{lib}");
        assert!(got.contains(&flag), "{flag} missing from {got:?}");
        assert!(!console.contains(&flag), "{flag} on a console link");
    }
    // ws32 is not needed: every drawing entry point is a pure virtual.
    assert!(!got.iter().any(|x| x.contains("ws32")), "{got:?}");

    // The 107 KB fix of experiment 77: euser and drtaeabi stay in front of the Rust
    // archive, whatever else the UI adds behind them.
    let pos =
        |args: &[String], needle: &str| args.iter().position(|x| x == needle).unwrap_or(usize::MAX);
    let archive = arg(a);
    assert!(pos(&got, "-l:euser.dso") < pos(&got, &archive), "{got:?}");
    assert!(
        pos(&got, "-l:drtaeabi.dso") < pos(&got, &archive),
        "{got:?}"
    );
    // And the Avkon libraries come after it, where an MMP's LIBRARY list would be.
    assert!(pos(&got, "-l:avkon.dso") > pos(&got, &archive), "{got:?}");
    assert!(pos(&got, "-l:avkon.dso") < pos(&got, "-lsupc++"), "{got:?}");
}

/// The Avkon subclasses are compiled for a `[ui]` project and for no other.
#[test]
fn the_s60_shim_is_only_built_for_a_ui_project() {
    let b = gui();
    let console = b.sdk.shim_sources(false).unwrap();
    let with_ui = b.sdk.shim_sources(true).unwrap();
    assert!(
        !console.iter().any(|s| s.ends_with("symrs_avkon.cpp")),
        "{console:?}"
    );
    assert!(
        with_ui.iter().any(|s| s.ends_with("symrs_avkon.cpp")),
        "{with_ui:?}"
    );
    // A `[ui]` build is the console list, unchanged and in order, plus the whole of
    // `shims/s60` — which is more than one file since the notes joined the subclasses.
    assert_eq!(&with_ui[..console.len()], &console[..], "{with_ui:?}");
    let s60 = b.sdk.ui_shim_dir();
    assert!(with_ui.len() > console.len(), "{with_ui:?}");
    assert!(
        with_ui[console.len()..]
            .iter()
            .all(|s| s.parent() == Some(s60.as_path())),
        "{with_ui:?}"
    );
}

/// What the Avkon headers need beyond the recorded C++ argv: the case-fold overlay
/// (`fbs.h` includes `FbsMessage.h`, which is `fbsmessage.h` on disk) and the UID3
/// the shim's `AppDllUid` returns.
#[test]
fn the_s60_shim_gets_the_casefold_overlay_and_the_uid() {
    let b = gui();
    let sources = b.sdk.shim_sources(true).unwrap();
    let source = sources
        .iter()
        .find(|s| s.ends_with("symrs_avkon.cpp"))
        .unwrap();
    let obj = Path::new("/p/build/shims/symrs_avkon.o");
    let overlay = Path::new("/p/build/sdk-include-casefold");
    let got = b.shim_compile_args(source, obj, Some(overlay)).unwrap();
    let uid = format!("-DSYMRS_UID3=0x{:08x}", b.gcce.uid3);
    assert!(got.contains(&uid), "{uid} missing from {got:?}");
    assert!(got.contains(&arg(overlay)), "{got:?}");
    // The source directory is its own, so `#include "symrs_avkon.h"` resolves.
    assert!(got.contains(&arg(&b.sdk.ui_shim_dir())), "{got:?}");
    // A console project's shim gets neither.
    let plain = rust().shim_compile_args(source, obj, None).unwrap();
    assert!(
        !plain.iter().any(|x| x.starts_with("-DSYMRS_UID3")),
        "{plain:?}"
    );
    assert!(!plain.contains(&arg(overlay)), "{plain:?}");
}
