use std::path::Path;

use super::*;

#[test]
fn link_args_append_mmp_libraries_once() {
    let d = fake();
    let args = d.link_args(
        "gui",
        Path::new("/p/build/gui.o"),
        Path::new("/p/build/gui.elf"),
        Path::new("/p/build/gui.exe.map"),
        &["euser.dso".into(), "avkon.dso".into()],
    );
    assert_eq!(args.iter().filter(|a| *a == "-l:euser.dso").count(), 1);
    let avkon = args.iter().position(|a| a == "-l:avkon.dso").unwrap();
    let supcpp = args.iter().position(|a| a == "-lsupc++").unwrap();
    assert!(avkon < supcpp);
}

#[test]
fn link_args_use_ld_2_29_1_recorded_experiment_5() {
    let d = fake();
    let args = d.link_args(
        "hello",
        Path::new("/proj/build/hello.o"),
        Path::new("/proj/build/hello.elf"),
        Path::new("/proj/build/hello.exe.map"),
        &[],
    );
    assert_eq!(
        args.first().map(String::as_str),
        Some("/gcc/binutils/bin/arm-none-symbianelf-ld")
    );
    assert_eq!(
        args,
        s(&[
            "/gcc/binutils/bin/arm-none-symbianelf-ld",
            "-L/gcc/lib/gcc/arm-none-symbianelf/12.1.0/",
            "-L",
            "/gcc/arm-none-symbianelf/lib",
            "--target1-abs",
            "--no-undefined",
            "-nostdlib",
            "-shared",
            "-Ttext",
            "0x8000",
            "-Tdata",
            "0x400000",
            "--default-symver",
            "-soname",
            "hello{000a0000}[e79e4cf9].exe",
            "--target1-abs",
            "--no-undefined",
            "-nostdlib",
            "--strip-debug",
            "--entry",
            "_E32Startup",
            "-u",
            "_E32Startup",
            "-L/sdk/epoc32/release/armv5/urel",
            "-l:eexe.lib",
            "-o",
            "/proj/build/hello.elf",
            "-Map",
            "/proj/build/hello.exe.map",
            "/proj/build/hello.o",
            "-(",
            "-l:usrt2_2.lib",
            "-)",
            "-L/gcc/arm-none-symbianelf/lib",
            "-L/sdk/epoc32/release/armv5/lib",
            "-l:euser.dso",
            "-l:dfpaeabi.dso",
            "-l:dfprvct2_2.dso",
            "-l:drtaeabi.dso",
            "-l:scppnwdl.dso",
            "-l:drtrvct2_2.dso",
            "-lsupc++",
            "-lgcc",
        ])
    );
    assert!(
        !args
            .iter()
            .any(|a| a.contains("-fPIC") || a.contains("-fPIE"))
    );
    assert!(
        !args
            .iter()
            .any(|a| a.contains("gcc-12.1.0/bin/arm-none-symbianelf-ld"))
    );
}
