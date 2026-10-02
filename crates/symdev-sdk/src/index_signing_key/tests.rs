use super::IndexSigningKey;
use crate::TrustedKeys;

const SEED_7: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";

#[test]
fn trusts_its_own_public_key() {
    let key = IndexSigningKey::from_base64(SEED_7).unwrap();
    assert_eq!(
        key.trusted(),
        TrustedKeys::parse("6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw=").unwrap()
    );
}

#[test]
fn a_trailing_newline_from_an_env_file_is_ignored() {
    let key = IndexSigningKey::from_base64(&format!("{SEED_7}\n")).unwrap();
    assert_eq!(
        key.trusted(),
        IndexSigningKey::from_base64(SEED_7).unwrap().trusted()
    );
}

#[test]
fn a_seed_that_is_not_32_bytes_of_base64_is_refused_without_echoing_it() {
    for seed in [
        "",
        "c2VjcmV0",
        "not base64 at all!",
        &format!("{SEED_7}AAAA"),
    ] {
        let e = IndexSigningKey::from_base64(seed).unwrap_err().to_string();
        assert!(e.contains("32-byte Ed25519 seed"), "{e}");
        if !seed.is_empty() {
            assert!(!e.contains(seed), "the error shows the secret: {e}");
        }
    }
}

#[test]
fn debug_shows_the_public_key_only() {
    let key = IndexSigningKey::from_base64(SEED_7).unwrap();
    let shown = format!("{key:?}");
    assert!(!shown.contains(SEED_7.trim_end_matches('=')), "{shown}");
    assert!(shown.contains(&key.trusted().fingerprints()[0]), "{shown}");
}
