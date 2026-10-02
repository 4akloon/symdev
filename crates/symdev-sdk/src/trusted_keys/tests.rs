use super::TrustedKeys;
use crate::{IndexSigningKey, SignedIndex};

/// The project key's public half, as the spec (§11) records it.
const PROJECT_KEY: &str = "C1yh60B72Qa4YE4rZOgoPJZmTYKbh/uzHjupoVwqfLU=";
const PROJECT_FINGERPRINT: &str =
    "bdf5345cc3ca8c30661dbc53b2cbd16983d081bf26913d0c3ebe7480ca334d44";
/// The public key of the seed of 32 bytes 0x07 (OpenSSL 3.5, `pkey -pubout`).
const KEY_7: &str = "6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw=";

fn key(byte: u8) -> IndexSigningKey {
    use base64::Engine;
    IndexSigningKey::from_base64(&base64::engine::general_purpose::STANDARD.encode([byte; 32]))
        .unwrap()
}

fn verifies(keys: &TrustedKeys, by: &IndexSigningKey) -> bool {
    let text = SignedIndex::sign("schema = 1\n", by).to_text();
    SignedIndex::split(&text).verify(keys, "file:///i").is_ok()
}

#[test]
fn the_builtin_keys_are_the_project_key() {
    let builtin = TrustedKeys::builtin();
    assert_eq!(builtin, TrustedKeys::parse(PROJECT_KEY).unwrap());
    assert_eq!(builtin.fingerprints(), [PROJECT_FINGERPRINT]);
}

#[test]
fn builtin_in_sources_toml_means_the_embedded_keys() {
    assert_eq!(
        TrustedKeys::parse("builtin").unwrap(),
        TrustedKeys::builtin()
    );
}

#[test]
fn a_base64_key_verifies_what_its_seed_signed() {
    let keys = TrustedKeys::parse(KEY_7).unwrap();
    assert_eq!(keys, key(7).trusted());
    assert!(verifies(&keys, &key(7)));
    assert!(!verifies(&keys, &key(8)));
    assert!(!verifies(&TrustedKeys::builtin(), &key(7)));
}

#[test]
fn either_key_verifies_during_a_rotation() {
    let keys = key(7).trusted().and(key(8).trusted());
    assert!(verifies(&keys, &key(7)));
    assert!(verifies(&keys, &key(8)));
    assert!(!verifies(&keys, &key(9)));
    assert_eq!(keys.fingerprints().len(), 2);
    assert_eq!(
        keys.clone().and(key(7).trusted()),
        keys,
        "a key is listed once"
    );
}

#[test]
fn anything_else_is_refused_with_both_forms_named() {
    for text in [
        "",
        "Builtin",
        "not base64!",
        "AAAA",
        // 33 bytes
        "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcH",
    ] {
        let e = TrustedKeys::parse(text).unwrap_err().to_string();
        assert!(
            e.contains("\"builtin\"") && e.contains("32-byte Ed25519 public key"),
            "{text:?}: {e}"
        );
    }
}

#[test]
fn surrounding_whitespace_is_not_part_of_a_key() {
    assert_eq!(
        TrustedKeys::parse(&format!(" {KEY_7}\n")).unwrap(),
        key(7).trusted()
    );
}
