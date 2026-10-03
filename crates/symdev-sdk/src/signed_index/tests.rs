use super::SignedIndex;
use crate::{Index, IndexSigningKey, SdkError, TrustedKeys};

const URL: &str = "https://pub-1.r2.dev/index.toml";
const BODY: &str = "schema = 1\n";
/// The base64 of 32 bytes 0x07 and of 32 bytes 0x08: two throwaway seeds.
const SEED_7: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
const SEED_8: &str = "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg=";
/// What OpenSSL 3.5 makes of BODY with SEED_7 (`openssl pkeyutl -sign -rawin`): the format
/// is plain RFC 8032 Ed25519, so install.sh's `openssl pkeyutl -verify` reads it.
const OPENSSL_SIGNATURE: &str =
    "c8Cm7FfNyA8i173wmilKFXuYaL1I/fZH7O2OvWWEMr7FbNAU8fY7rHx2zGXzI/PKsOEMDKXT44pbq40e5fMpDQ==";

const PACKAGE: &str = "schema = 1\n\n[[package]]\nid = \"gcce;12.1.0\"\nlicense = \"GPL-3.0-or-later\"\n\
                       depends = []\n\n[[package.archive]]\nhost = \"x86_64-linux\"\n\
                       url = \"gcce/12.1.0/a.tar.gz\"\nsha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n\
                       size = 1\n";

fn key(seed: &str) -> IndexSigningKey {
    IndexSigningKey::from_base64(seed).unwrap()
}

/// `verify`'s error, which must be an untrusted-index error naming `URL`.
fn refused(text: &str, keys: &TrustedKeys) -> String {
    match SignedIndex::split(text).verify(keys, URL) {
        Err(e @ SdkError::UntrustedIndex { .. }) => {
            let message = e.to_string();
            assert!(message.contains(URL), "{message}");
            message
        }
        other => panic!("expected UntrustedIndex for {text:?}, got {other:?}"),
    }
}

#[test]
fn the_signature_is_the_first_line_and_covers_the_bytes_after_it() {
    let text = SignedIndex::sign(BODY, &key(SEED_7)).to_text();
    assert_eq!(
        text,
        format!("# symdev-signature: ed25519 {OPENSSL_SIGNATURE}\n{BODY}")
    );
}

#[test]
fn a_signed_index_verifies_with_the_signers_key() {
    let text = SignedIndex::sign(PACKAGE, &key(SEED_7)).to_text();
    let signed = SignedIndex::split(&text);
    assert!(signed.is_signed());
    assert_eq!(signed.body(), PACKAGE);
    assert_eq!(signed.verify(&key(SEED_7).trusted(), URL).unwrap(), PACKAGE);
}

#[test]
fn a_changed_body_is_refused_as_tampered() {
    let text = SignedIndex::sign(PACKAGE, &key(SEED_7))
        .to_text()
        .replace("size = 1", "size = 2");
    let e = refused(&text, &key(SEED_7).trusted());
    assert!(
        e.contains("does not verify") && e.contains("tampered"),
        "{e}"
    );
}

#[test]
fn a_signature_by_another_key_is_refused() {
    let text = SignedIndex::sign(PACKAGE, &key(SEED_8)).to_text();
    let e = refused(&text, &key(SEED_7).trusted());
    assert!(e.contains("does not verify"), "{e}");
}

#[test]
fn an_unsigned_index_is_refused_where_a_signature_is_required() {
    let signed = SignedIndex::split(PACKAGE);
    assert!(!signed.is_signed());
    assert_eq!(signed.body(), PACKAGE);
    let e = refused(PACKAGE, &TrustedKeys::builtin());
    assert!(e.contains("unsigned"), "{e}");
}

#[test]
fn only_the_first_line_can_hold_the_signature() {
    let text = format!("{BODY}# symdev-signature: ed25519 {OPENSSL_SIGNATURE}\n");
    assert!(!SignedIndex::split(&text).is_signed());
    refused(&text, &key(SEED_7).trusted());
}

#[test]
fn a_malformed_signature_line_is_refused() {
    for line in [
        "# symdev-signature: ed25519 not-base64!".to_string(),
        "# symdev-signature: rsa AAAA".into(),
        "# symdev-signature: ed25519 AAAA".into(),
        format!("# symdev-signature: ed25519 {OPENSSL_SIGNATURE}\r"),
        format!("# symdev-signature: ed25519  {OPENSSL_SIGNATURE}"),
    ] {
        let text = format!("{line}\n{BODY}");
        assert!(SignedIndex::split(&text).is_signed(), "{line}");
        let e = refused(&text, &key(SEED_7).trusted());
        assert!(e.contains("signature line"), "{line}: {e}");
    }
}

#[test]
fn a_signature_line_without_a_body_is_signed_and_empty() {
    let text = format!("# symdev-signature: ed25519 {OPENSSL_SIGNATURE}");
    let signed = SignedIndex::split(&text);
    assert!(signed.is_signed());
    assert_eq!(signed.body(), "");
    refused(&text, &key(SEED_7).trusted());
}

/// symdev 0.1.0 parses the whole text: to it the signature line is a TOML comment.
#[test]
fn a_signed_index_still_parses_for_symdev_0_1_0() {
    let text = SignedIndex::sign(PACKAGE, &key(SEED_7)).to_text();
    assert_eq!(
        Index::parse(&text, "public").unwrap(),
        Index::parse(PACKAGE, "public").unwrap()
    );
}

#[test]
fn split_and_to_text_give_back_the_served_bytes() {
    let signed = SignedIndex::sign(PACKAGE, &key(SEED_7)).to_text();
    for text in [signed.as_str(), PACKAGE, ""] {
        assert_eq!(SignedIndex::split(text).to_text(), text);
    }
}

/// A checkout or an editor that converts line endings changes the signed bytes; that, more
/// likely than tampering, is why such an index fails (review 0.2.0, minor 8).
#[test]
fn crlf_line_endings_are_named_as_the_likely_cause() {
    let text = SignedIndex::sign(PACKAGE, &key(SEED_7))
        .to_text()
        .replace('\n', "\r\n");
    let e = refused(&text, &key(SEED_7).trusted());
    assert!(e.contains("CRLF"), "{e}");
}

#[test]
fn a_byte_order_mark_is_named_as_the_likely_cause() {
    let signed = SignedIndex::sign(PACKAGE, &key(SEED_7)).to_text();
    let e = refused(&format!("\u{feff}{signed}"), &key(SEED_7).trusted());
    assert!(e.contains("byte-order mark"), "{e}");
}

/// Neither is named where it is not the cause.
#[test]
fn other_refusals_name_neither_line_endings_nor_a_byte_order_mark() {
    for text in [
        PACKAGE.to_string(),
        format!("\u{feff}{PACKAGE}"),
        format!("# symdev-signature: ed25519 not-base64!\n{BODY}"),
    ] {
        let e = refused(&text, &key(SEED_7).trusted());
        assert!(!e.contains("CRLF") && !e.contains("byte-order"), "{e}");
    }
}
