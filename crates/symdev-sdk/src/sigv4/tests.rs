//! AWS Signature V4 against AWS's published examples.
//!
//! S3 examples (GET Object, PUT Object, GET Bucket Lifecycle, Get Bucket (List Objects)), from
//! the S3 API reference page "Signature Calculations for the Authorization Header:
//! Transferring Payload in a Single Chunk (AWS Signature Version 4)",
//! <https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html>. Since
//! 2026 that URL redirects to the API index, so the bytes were taken from the last archived
//! copy of the official page:
//! <https://web.archive.org/web/20251208134526id_/https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html>.
//!
//! Generic suite `get-vanilla` (service `service`, signs only `host;x-amz-date`): AWS's
//! `aws-sig-v4-test-suite` (the zip once at
//! `https://docs.aws.amazon.com/general/latest/gr/samples/aws-sig-v4-test-suite.zip`, now 404),
//! as kept by AWS in
//! <https://github.com/awslabs/aws-c-auth/tree/main/tests/aws-signing-test-suite/v4/get-vanilla>
//! (`context.json`, `header-canonical-request.txt`, `header-string-to-sign.txt`,
//! `header-signature.txt`, `header-signed-request.txt`).

use super::SigV4;
use super::canonical_request::CanonicalRequest;
use crate::{AmzDate, S3Keys};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// 2013-05-24T00:00:00Z, the S3 examples' request time.
const S3_EXAMPLE_TIME: u64 = 1_369_353_600;

fn s3_example_signer() -> SigV4 {
    SigV4::s3(
        S3Keys {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        },
        "us-east-1",
    )
}

fn header<'a>(signed: &'a [(String, String)], name: &str) -> &'a str {
    signed
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("no {name} in {signed:?}"))
}

fn owned(headers: &[(&str, &str)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
}

#[test]
fn s3_get_object_matches_the_published_canonical_request_string_and_signature() {
    let now = AmzDate::from_unix(S3_EXAMPLE_TIME);
    let headers = owned(&[
        ("host", "examplebucket.s3.amazonaws.com"),
        ("range", "bytes=0-9"),
        ("x-amz-content-sha256", EMPTY_SHA256),
        ("x-amz-date", "20130524T000000Z"),
    ]);
    let url = "https://examplebucket.s3.amazonaws.com/test.txt";
    let canonical = CanonicalRequest::new("GET", url, &headers, EMPTY_SHA256).unwrap();
    assert_eq!(
        canonical.text(),
        "GET\n/test.txt\n\nhost:examplebucket.s3.amazonaws.com\nrange:bytes=0-9\n\
         x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n\
         x-amz-date:20130524T000000Z\n\nhost;range;x-amz-content-sha256;x-amz-date\n\
         e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let signer = s3_example_signer();
    let string_to_sign = signer.string_to_sign(&now, &canonical);
    assert_eq!(
        string_to_sign,
        "AWS4-HMAC-SHA256\n20130524T000000Z\n20130524/us-east-1/s3/aws4_request\n\
         7344ae5b7ee6c3e7e6b0fe0640412a37625d1fbfff95c48bbb2dc43964946972"
    );
    assert_eq!(
        signer.signature(&now, &string_to_sign).unwrap(),
        "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn sign_reproduces_the_s3_get_object_authorization() {
    let signed = s3_example_signer()
        .sign(
            "GET",
            "https://examplebucket.s3.amazonaws.com/test.txt",
            &[("Range", "bytes=0-9")],
            EMPTY_SHA256,
            &AmzDate::from_unix(S3_EXAMPLE_TIME),
        )
        .unwrap();
    let names: Vec<&str> = signed.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["x-amz-date", "x-amz-content-sha256", "authorization"]
    );
    assert_eq!(header(&signed, "x-amz-date"), "20130524T000000Z");
    assert_eq!(header(&signed, "x-amz-content-sha256"), EMPTY_SHA256);
    // The page prints the same value without a space after each comma.
    assert_eq!(
        header(&signed, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
         SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
         Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn sign_reproduces_the_s3_put_object_example_with_a_dollar_in_the_key() {
    let payload = "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072";
    let signed = s3_example_signer()
        .sign(
            "PUT",
            "https://examplebucket.s3.amazonaws.com/test$file.text",
            &[
                ("Date", "Fri, 24 May 2013 00:00:00 GMT"),
                ("x-amz-storage-class", "REDUCED_REDUNDANCY"),
            ],
            payload,
            &AmzDate::from_unix(S3_EXAMPLE_TIME),
        )
        .unwrap();
    assert_eq!(
        header(&signed, "authorization"),
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
         SignedHeaders=date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class, \
         Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
    );
}

#[test]
fn s3_put_object_canonical_request_encodes_the_dollar_once() {
    let headers = owned(&[("host", "examplebucket.s3.amazonaws.com")]);
    let url = "https://examplebucket.s3.amazonaws.com/test$file.text";
    let canonical = CanonicalRequest::new("PUT", url, &headers, EMPTY_SHA256).unwrap();
    assert!(canonical.text().starts_with("PUT\n/test%24file.text\n\n"));
}

#[test]
fn sign_reproduces_the_s3_get_bucket_lifecycle_example() {
    let signed = s3_example_signer()
        .sign(
            "GET",
            "https://examplebucket.s3.amazonaws.com/?lifecycle",
            &[],
            EMPTY_SHA256,
            &AmzDate::from_unix(S3_EXAMPLE_TIME),
        )
        .unwrap();
    assert!(
        header(&signed, "authorization").ends_with(
            "SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=fea454ca298b7da1c68078a5d1bdbfbbe0d65c699e0f91ac7a200a0136783543"
        ),
        "{signed:?}"
    );
}

#[test]
fn sign_reproduces_the_s3_list_objects_example() {
    let signed = s3_example_signer()
        .sign(
            "GET",
            "https://examplebucket.s3.amazonaws.com/?max-keys=2&prefix=J",
            &[],
            EMPTY_SHA256,
            &AmzDate::from_unix(S3_EXAMPLE_TIME),
        )
        .unwrap();
    assert!(
        header(&signed, "authorization").ends_with(
            "Signature=34b48302e7b5fa45bde8084f4b7868a86f0a534bc59db6670ed5711ef69dc6f7"
        ),
        "{signed:?}"
    );
}

#[test]
fn get_vanilla_from_the_generic_test_suite() {
    let signer = SigV4 {
        keys: S3Keys {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
        },
        region: "us-east-1".into(),
        service: "service".into(),
    };
    let now = AmzDate::from_unix(1_440_938_160);
    let headers = owned(&[
        ("Host", "example.amazonaws.com"),
        ("X-Amz-Date", "20150830T123600Z"),
    ]);
    let url = "https://example.amazonaws.com/";
    let canonical = CanonicalRequest::new("GET", url, &headers, EMPTY_SHA256).unwrap();
    assert_eq!(
        canonical.text(),
        "GET\n/\n\nhost:example.amazonaws.com\nx-amz-date:20150830T123600Z\n\nhost;x-amz-date\n\
         e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        signer.string_to_sign(&now, &canonical),
        "AWS4-HMAC-SHA256\n20150830T123600Z\n20150830/us-east-1/service/aws4_request\n\
         bb579772317eb040ac9ed261061d46c1f17a8133879d6129b6e1c25292927e63"
    );
    assert_eq!(
        signer.authorization(&now, &canonical).unwrap(),
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
         SignedHeaders=host;x-amz-date, \
         Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
    );
}

#[test]
fn header_values_are_trimmed_and_inner_spaces_collapsed() {
    let headers = owned(&[("host", "h"), ("X-Meta", "  a   b  ")]);
    let canonical = CanonicalRequest::new("GET", "https://h/", &headers, EMPTY_SHA256).unwrap();
    assert!(
        canonical.text().contains("\nx-meta:a b\n"),
        "{}",
        canonical.text()
    );
}

#[test]
fn refuses_an_extra_header_the_signer_sets_itself() {
    let now = AmzDate::from_unix(S3_EXAMPLE_TIME);
    let err = s3_example_signer()
        .sign(
            "GET",
            "https://h/x",
            &[("X-Amz-Date", "1")],
            EMPTY_SHA256,
            &now,
        )
        .unwrap_err();
    assert!(err.to_string().contains("x-amz-date"), "{err}");
}

#[test]
fn refuses_a_url_it_cannot_sign() {
    let now = AmzDate::from_unix(S3_EXAMPLE_TIME);
    let err = s3_example_signer()
        .sign("GET", "https://h/a%zzb", &[], EMPTY_SHA256, &now)
        .unwrap_err();
    assert!(err.to_string().contains("https://h/a%zzb"), "{err}");
}
