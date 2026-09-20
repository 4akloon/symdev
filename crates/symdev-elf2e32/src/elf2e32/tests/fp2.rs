//! Test against a real FP2 SDK's DSOs, when `SYMDEV_EPOCROOT` points at one.
use super::*;

#[test]
fn fp2_dso_ordinals_match_experiment_44_table() {
    // Reads SDK DSOs only when SYMDEV_EPOCROOT points at an FP2 SDK; skipped otherwise.
    let Some(root) = std::env::var_os("SYMDEV_EPOCROOT") else {
        return;
    };
    let libpath = PathBuf::from(root).join("epoc32/release/armv5/lib");
    if !libpath.is_dir() {
        return;
    }
    let job = Elf2E32 {
        libpath,
        ..experiment_6()
    };
    let ordinals = job.ordinals(&hello_elf()).unwrap();
    for line in include_str!("../../testdata/hello_ordinals.txt").lines() {
        if line.starts_with('#') {
            continue;
        }
        let mut f = line.split_whitespace();
        let (dll, symbol, want) = (f.next().unwrap(), f.next().unwrap(), f.next().unwrap());
        let want = u32::from_str_radix(want.trim_start_matches("0x"), 16).unwrap();
        assert_eq!(ordinals.get(dll, symbol), Some(want), "{dll} {symbol}");
    }
}
