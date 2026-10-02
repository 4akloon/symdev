//! Which index a source accepts: any (no `key`), or only one its keys signed.

use super::SourceSpec;
use crate::{Auth, Index, IndexSigningKey, SdkError, SignedIndex, TrustedKeys};

/// The base64 of 32 bytes 0x07 and 0x08: throwaway seeds.
const SEED_7: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
const SEED_8: &str = "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg=";
const EMPTY: &str = "schema = 1\n";

fn signed(body: &str, seed: &str) -> String {
    SignedIndex::sign(body, &IndexSigningKey::from_base64(seed).unwrap()).to_text()
}

fn trusting(seed: &str) -> SourceSpec {
    let keys = IndexSigningKey::from_base64(seed).unwrap().trusted();
    SourceSpec::new("mirror", "https://mirror.example/", Auth::None)
        .unwrap()
        .with_key(keys)
}

fn untrusted(source: &SourceSpec, text: &str) -> String {
    match source.parse_index(text) {
        Err(e @ SdkError::UntrustedIndex { .. }) => {
            let message = e.to_string();
            assert!(message.contains(&source.index_url()), "{message}");
            message
        }
        other => panic!("expected UntrustedIndex for {text:?}, got {other:?}"),
    }
}

#[test]
fn the_built_in_source_accepts_only_an_index_signed_by_symdevs_key() {
    let source = SourceSpec::builtin().unwrap();
    assert_eq!(source.key, Some(TrustedKeys::builtin()));
    assert!(untrusted(&source, EMPTY).contains("unsigned"));
    assert!(untrusted(&source, &signed(EMPTY, SEED_7)).contains("does not verify"));
}

#[test]
fn a_source_without_a_key_reads_any_index_as_symdev_0_1_0_did() {
    let source = SourceSpec::new("mirror", "https://mirror.example/", Auth::None).unwrap();
    assert_eq!(source.key, None);
    for text in [
        EMPTY.to_string(),
        signed(EMPTY, SEED_7),
        format!("# symdev-signature: rsa AAAA\n{EMPTY}"),
    ] {
        assert_eq!(source.parse_index(&text).unwrap(), Index::empty(), "{text}");
    }
}

#[test]
fn a_source_with_a_key_reads_only_what_that_key_signed() {
    let source = trusting(SEED_7);
    assert_eq!(
        source.parse_index(&signed(EMPTY, SEED_7)).unwrap(),
        Index::empty()
    );
    assert!(untrusted(&source, &signed(EMPTY, SEED_8)).contains("does not verify"));
    assert!(untrusted(&source, EMPTY).contains("unsigned"));
    let tampered = signed(EMPTY, SEED_7) + "\n[[package]]\n";
    assert!(untrusted(&source, &tampered).contains("tampered"));
}

#[test]
fn a_signed_index_is_still_checked_as_an_index() {
    let e = trusting(SEED_7)
        .parse_index(&signed("schema = 2\n", SEED_7))
        .unwrap_err();
    assert!(matches!(e, SdkError::UnknownSchema { found: 2, .. }), "{e}");
}
