//! `symdev sdk list --offline` reads only the receipts: what only a download needs — a
//! readable `sources.toml`, whole key pairs — cannot stop it.

use predicates::prelude::*;

mod common;
use common::repo::World;

#[test]
fn offline_listing_ignores_what_only_a_download_needs() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.bin()
        .args(["sdk", "install", "gcce;12.1.0"])
        .assert()
        .success();
    w.sources("builtin = maybe\n");
    w.bin()
        .args(["sdk", "list", "--offline"])
        .assert()
        .success()
        .stdout("installed  gcce;12.1.0  (local)\n");
    w.sources("[[source]]\nname = \"private\"\nurl = \"https://127.0.0.1:1/b/\"\nauth = \"s3\"\n");
    w.bin()
        .args(["sdk", "list", "--offline"])
        .env("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "AKID")
        .assert()
        .success()
        .stdout("installed  gcce;12.1.0  (local)\n")
        .stderr(predicate::str::is_empty());
}
