use std::path::{Path, PathBuf};

use super::*;

#[test]
fn compile_includes_wrap_the_sdk_include_dir() {
    let d = fake();
    let inc = CompileIncludes {
        user: vec![PathBuf::from("/p/build")],
        system: vec![PathBuf::from("/p/build/sdk-include-casefold")],
        prefix: Vec::new(),
    };
    let args = d
        .compile_args(
            Path::new("/p/src"),
            &inc,
            Path::new("/p/src/gui.cpp"),
            Path::new("/p/build/gui.o"),
        )
        .unwrap();
    let at = |v: &str| args.iter().position(|a| a == v).unwrap();
    assert!(at("/p/src") < at("/p/build"));
    assert!(at("/p/build") < at("/sdk/epoc32/include"));
    assert!(at("/sdk/epoc32/include/variant") < at("/p/build/sdk-include-casefold"));
}

#[test]
fn compile_args_match_recorded_experiment_5() {
    let d = fake();
    let args = d
        .compile_args(
            Path::new("/proj"),
            &CompileIncludes::default(),
            Path::new("/proj/hello.cpp"),
            Path::new("/proj/build/hello.o"),
        )
        .unwrap();
    assert_eq!(
        args,
        s(&[
            "/gcc/bin/arm-none-symbianelf-g++",
            "-O2",
            "-fexceptions",
            "-march=armv5t",
            "-mapcs",
            "-mthumb-interwork",
            "-mthumb",
            "-msoft-float",
            "-fpermissive",
            "-Wno-narrowing",
            "-D__SYMBIAN32__",
            "-D__EPOC32__",
            "-D__MARM__",
            "-D__GCCE__",
            "-D__EXE__",
            "-include",
            "/sdk/epoc32/include/gcce/gcce.h",
            "-D__PRODUCT_INCLUDE__=\"/sdk/epoc32/include/variant/symbian_os_v9.3.hrh\"",
            "-nostdinc",
            "-c",
            "-D__MARM_THUMB__",
            "-D__MARM_INTERWORK__",
            "-DNDEBUG",
            "-D_UNICODE",
            "-D__S60_3X__",
            "-D__SERIES60_3X__",
            "-D__EABI__",
            "-D__MARM_ARMV5__",
            "-D__SUPPORT_CPP_EXCEPTIONS__",
            "-I",
            "/proj",
            "-I",
            "/sdk/epoc32/include",
            "-I",
            "/sdk/epoc32/include/variant",
            "-I",
            "/gcc/lib/gcc/arm-none-symbianelf/12.1.0/include",
            "-o",
            "/proj/build/hello.o",
            "/proj/hello.cpp",
        ])
    );
    assert!(
        !args
            .iter()
            .any(|a| a.contains("-fPIC") || a.contains("-fPIE"))
    );
}

/// §8.2 item 6 and §8.4 position 12: `MACRO` joins the `-D` list in source order and
/// `OPTION GCCE` lands after the architecture flags, before the instruction set.
#[test]
fn macro_and_option_gcce_reach_the_command_line() {
    let d = fake();
    let mmp = crate::Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nMACRO COMBINED\nMACRO NO_TGZ\nOPTION GCCE -O3\n",
    )
    .unwrap();
    let args = d
        .compile_args_for(
            &d.exe_module(),
            &CompileFlags::of(&mmp),
            Path::new("/proj"),
            &CompileIncludes::default(),
            Path::new("/proj/a.cpp"),
            Path::new("/proj/build/a.o"),
        )
        .unwrap();
    let at = |v: &str| args.iter().position(|a| a == v).unwrap();
    assert!(at("-mapcs") < at("-O3"));
    assert!(at("-O3") < at("-mthumb"));
    assert!(at("-D__MARM_ARMV5__") < at("-DCOMBINED"));
    assert!(at("-DCOMBINED") < at("-DNO_TGZ"));
    assert!(at("-DNO_TGZ") < at("-D__SUPPORT_CPP_EXCEPTIONS__"));
}
