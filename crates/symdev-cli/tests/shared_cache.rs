//! Parallel `symdev sdk install`s into different `SYMDEV_HOME`s that share one download
//! cache (`XDG_CACHE_HOME`) all succeed.

use std::process::{Command, Stdio};

use symdev_sdk::Host;

mod common;
use common::repo::World;

#[test]
fn processes_with_their_own_homes_share_one_cache() {
    const PROCESSES: usize = 6;
    let mut w = World::new();
    // Big enough (about 4 MB packed) that the downloads overlap.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let noise: String = (0..8_000_000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            char::from_digit((seed % 16) as u32, 16).unwrap_or('0')
        })
        .collect();
    w.add(
        "gcce;12.1.0",
        Host::X86_64Linux,
        &[("noise", &noise, false)],
    );
    let cache = w.tmp.path().join("shared-cache");
    let children: Vec<_> = (0..PROCESSES)
        .map(|n| {
            Command::new(assert_cmd::cargo::cargo_bin("symdev"))
                .args(["sdk", "install", "gcce;12.1.0"])
                .env_clear()
                .env("SYMDEV_HOME", w.tmp.path().join(format!("home-{n}")))
                .env("XDG_CACHE_HOME", &cache)
                .env("XDG_CONFIG_HOME", w.tmp.path().join("config"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for (n, child) in children.into_iter().enumerate() {
        let out = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "process {n}: {stderr}");
        let noise_file = w.tmp.path().join(format!("home-{n}/gcce/12.1.0/noise"));
        assert_eq!(std::fs::metadata(noise_file).unwrap().len(), 8_000_000);
    }
}
