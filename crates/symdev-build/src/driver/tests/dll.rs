use std::path::{Path, PathBuf};

use super::*;
use crate::Mmp;

#[test]
fn dll_module_uses_the_sdk_dll_recipe() {
    let d = fake();
    let mmp =
        Mmp::parse("TARGET mathlib.dll\nTARGETTYPE DLL\nUID 0x1000008d 0xe5d1b001\nSOURCE m.cpp\n")
            .unwrap();
    let module = Module::of(&mmp, 0xe79e_4cf9, None).unwrap();
    assert!(module.dll);
    let args = d.elf2e32_args_for(
        &module,
        "mathlib",
        Path::new("/p/build/mathlib.elf"),
        Path::new("/p/build/mathlib.dll"),
        Some(Path::new("/p/build")),
        None,
    );
    for want in [
        "--sid=0xe5d1b001",
        "--uid1=0x10000079",
        "--uid2=0x1000008d",
        "--uid3=0xe5d1b001",
        "--targettype=DLL",
        "--ignorenoncallable",
        "--dso=/p/build/mathlib.dso",
        "--defoutput=/p/build/mathlib.def",
        "--linkas=mathlib{000a0000}[e5d1b001].dll",
        "--libpath=/sdk/epoc32/release/armv5/lib;/p/build",
    ] {
        assert!(args.iter().any(|a| a == want), "missing {want}: {args:?}");
    }
    let job = symdev_elf2e32::Elf2E32::from_args(&args).unwrap();
    assert_eq!(job.uid().uid2, 0x1000_008d);
    let link = d.link_args_for(
        &module,
        "mathlib",
        Path::new("/p/build/m.o"),
        Path::new("/p/build/mathlib.elf"),
        Path::new("/p/build/mathlib.dll.map"),
        &[],
        &[PathBuf::from("/p/build")],
    );
    assert!(link.iter().any(|a| a == "-l:edll.lib"));
    assert!(link.iter().any(|a| a == "_E32Dll"));
    assert!(link.iter().any(|a| a == "mathlib{000a0000}[e5d1b001].dll"));
    let l = link.iter().position(|a| a == "-L/p/build").unwrap();
    let euser = link.iter().position(|a| a == "-l:euser.dso").unwrap();
    assert!(l < euser);
    let compile = d
        .compile_args_for(
            &module,
            &CompileFlags::default(),
            Path::new("/p"),
            &CompileIncludes::default(),
            Path::new("/p/m.cpp"),
            Path::new("/p/build/m.o"),
        )
        .unwrap();
    assert!(compile.iter().any(|a| a == "-D__DLL__"));
    assert!(!compile.iter().any(|a| a == "-D__EXE__"));
}

#[test]
fn dll_module_needs_uid_line() {
    let mmp = Mmp::parse("TARGET m.dll\nTARGETTYPE DLL\nSOURCE m.cpp\n").unwrap();
    assert!(Module::of(&mmp, 1, None).is_err());
}

/// Experiment 66: the manifest's `secure_id` wins over the MMP's `SECUREID`, and an
/// absent one leaves the post-linker to default it to UID3.
#[test]
fn manifest_secure_id_overrides_the_mmp_one() {
    let mmp = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSECUREID 0xA000EF77\nSOURCE a.cpp\nSYSTEMINCLUDE \\epoc32\\include\n",
    )
    .unwrap();
    assert_eq!(mmp.secureid, Some(0xA000_EF77));
    assert_eq!(
        Module::of(&mmp, 0xe000_0001, Some(0xE000_EF77))
            .unwrap()
            .secureid,
        Some(0xE000_EF77)
    );
    assert_eq!(
        Module::of(&mmp, 0xe000_0001, None).unwrap().secureid,
        Some(0xA000_EF77)
    );
    let bare = Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\n").unwrap();
    assert_eq!(Module::of(&bare, 0xe000_0001, None).unwrap().secureid, None);
    assert_eq!(
        Module::of(&bare, 0xe000_0001, Some(0xE000_EF77))
            .unwrap()
            .secureid,
        Some(0xE000_EF77)
    );
}
