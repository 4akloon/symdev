use super::*;

#[test]
fn from_env_uses_sdk_fixture_and_default_wine() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::set_var("SYMDEV_EPOCROOT", "/sdk");
        std::env::remove_var("SYMDEV_WINE");
    }
    let tools = SisTools::from_env().unwrap();
    assert_eq!(tools.wine, PathBuf::from("/usr/bin/wine"));
    assert_eq!(
        tools.makesis,
        PathBuf::from("/sdk/epoc32/tools/makesis.exe")
    );
    assert_eq!(
        tools.signsis,
        PathBuf::from("/sdk/epoc32/tools/signsis.exe")
    );
}

#[test]
fn from_env_missing_epocroot_errors() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::remove_var("SYMDEV_EPOCROOT");
    }
    let err = match SisTools::from_env() {
        Err(e) => e,
        Ok(_) => panic!("expected missing SYMDEV_EPOCROOT"),
    };
    assert_eq!(err.to_string(), "missing toolchain: SYMDEV_EPOCROOT");
}

#[test]
fn makesis_args_match_experiment_7() {
    assert_eq!(
        sdk_tools().makesis_args("hello.pkg", "hello.sis"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makesis.exe",
            "-v",
            "hello.pkg",
            "hello.sis",
        ]
    );
}

#[test]
fn signsis_args_match_experiment_8() {
    assert_eq!(
        sdk_tools().signsis_args(
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret"
        ),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/signsis.exe",
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret",
        ]
    );
}
