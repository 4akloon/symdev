use super::Index;
use crate::{ArchiveEntry, Host, IndexPackage, PackageId, SdkError};

const GCCE_SHA: &str = "8c41aa5f0e8d6c7b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b";
const SDK_SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn spec_example() -> String {
    format!(
        r#"schema = 1

[[package]]
id = "gcce;12.1.0"
license = "GPL-3.0-or-later"
source-code = "src/gcce/12.1.0/3f9a.tar.gz"   # GPL corresponding source
depends = []                                 # exact ids, no ranges

[[package.archive]]
host = "x86_64-linux"
url = "gcce/12.1.0/{GCCE_SHA}.tar.gz"
sha256 = "{GCCE_SHA}"
size = 61234567

[[package]]
id = "sdk;s60-3rd-fp2;1.1"
license = "LicenseRef-Nokia-S60-SDK-EULA"

[[package.archive]]
host = "any"
url = "sdk/s60-3rd-fp2/1.1/{SDK_SHA}.tar.gz"
sha256 = "{SDK_SHA}"
size = 40000000
"#
    )
}

fn id(s: &str) -> PackageId {
    PackageId::parse(s).unwrap()
}

fn bad_index(text: &str) -> String {
    match Index::parse(text, "public") {
        Err(SdkError::BadIndex {
            source_name,
            detail,
        }) => {
            assert_eq!(source_name, "public");
            detail
        }
        other => panic!("expected BadIndex, got {other:?}"),
    }
}

#[test]
fn parses_the_spec_example() {
    let index = Index::parse(&spec_example(), "public").unwrap();
    assert_eq!(index.schema, 1);
    assert_eq!(index.packages.len(), 2);
    let gcce = index.find(&id("gcce;12.1.0")).unwrap();
    assert_eq!(gcce.license, "GPL-3.0-or-later");
    assert_eq!(
        gcce.source_code.as_deref(),
        Some("src/gcce/12.1.0/3f9a.tar.gz")
    );
    assert!(gcce.depends.is_empty());
    assert_eq!(
        gcce.archives,
        [ArchiveEntry {
            host: Host::X86_64Linux,
            url: format!("gcce/12.1.0/{GCCE_SHA}.tar.gz"),
            sha256: GCCE_SHA.to_string(),
            size: 61234567,
        }]
    );
    let sdk = index.find(&id("sdk;s60-3rd-fp2;1.1")).unwrap();
    assert_eq!(sdk.source_code, None);
    assert_eq!(sdk.archives[0].host, Host::Any);
    assert!(index.find(&id("gcce;14.2.0")).is_none());
}

#[test]
fn a_newer_schema_tells_the_user_to_update() {
    let text = spec_example().replace("schema = 1", "schema = 2");
    match Index::parse(&text, "public") {
        Err(e @ SdkError::UnknownSchema { found: 2, .. }) => {
            assert!(e.to_string().contains("update symdev"), "{e}");
            assert!(e.to_string().contains("`public`"), "{e}");
        }
        other => panic!("expected UnknownSchema, got {other:?}"),
    }
}

#[test]
fn a_missing_schema_is_a_bad_index() {
    let text = spec_example().replace("schema = 1", "");
    assert!(bad_index(&text).contains("schema"));
}

#[test]
fn toml_syntax_errors_are_a_bad_index() {
    bad_index("schema = 1\n[[package]\n");
}

#[test]
fn a_sha256_that_is_not_64_lowercase_hex_is_a_bad_index() {
    let upper = spec_example().replace(
        &format!("sha256 = \"{GCCE_SHA}\""),
        &format!("sha256 = \"{}\"", GCCE_SHA.to_uppercase()),
    );
    assert!(bad_index(&upper).contains("sha256"));
    let short = spec_example().replace(&format!("sha256 = \"{SDK_SHA}\""), "sha256 = \"abc\"");
    assert!(bad_index(&short).contains("sdk;s60-3rd-fp2;1.1"));
}

#[test]
fn an_unsafe_archive_url_is_a_bad_index() {
    let text = spec_example().replace(
        &format!("url = \"gcce/12.1.0/{GCCE_SHA}.tar.gz\""),
        "url = \"https://evil.example/x.tar.gz\"",
    );
    assert!(bad_index(&text).contains("https://evil.example/x.tar.gz"));
    let source = spec_example().replace("src/gcce/12.1.0/3f9a.tar.gz", "../x.tar.gz");
    assert!(bad_index(&source).contains("../x.tar.gz"));
}

#[test]
fn a_package_listed_twice_is_a_bad_index() {
    let text = format!("{0}\n{1}", spec_example(), &spec_example()[10..]);
    assert!(bad_index(&text).contains("gcce;12.1.0"));
}

#[test]
fn a_package_without_archives_is_a_bad_index() {
    let text = "schema = 1\n[[package]]\nid = \"gcce;1.0\"\nlicense = \"MIT\"\n";
    assert!(bad_index(text).contains("no archive"));
}

#[test]
fn to_toml_round_trips_with_packages_sorted_by_id() {
    let mut index = Index::parse(&spec_example(), "public").unwrap();
    index.packages.reverse();
    let text = index.to_toml().unwrap();
    let again = Index::parse(&text, "public").unwrap();
    index.packages.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(again, index);
    assert!(text.find("gcce;12.1.0").unwrap() < text.find("sdk;s60-3rd-fp2;1.1").unwrap());
    assert!(text.contains("source-code = "), "{text}");
}

#[test]
fn empty_is_schema_one_without_packages() {
    let index = Index::empty();
    assert_eq!(index.schema, 1);
    assert!(index.packages.is_empty());
    assert_eq!(Index::parse(&index.to_toml().unwrap(), "x").unwrap(), index);
}

fn package(name: &str) -> IndexPackage {
    IndexPackage {
        id: id(name),
        license: "MIT".into(),
        source_code: None,
        depends: vec![],
        archives: vec![ArchiveEntry {
            host: Host::Any,
            url: format!("a/{SDK_SHA}.tar.gz"),
            sha256: SDK_SHA.into(),
            size: 1,
        }],
    }
}

#[test]
fn insert_adds_a_new_id_and_refuses_a_published_one() {
    let mut index = Index::empty();
    index.insert(package("gcce;12.1.0")).unwrap();
    assert!(index.find(&id("gcce;12.1.0")).is_some());
    let e = index.insert(package("gcce;12.1.0")).unwrap_err();
    let text = e.to_string();
    assert!(text.contains("gcce;12.1.0"), "{text}");
    assert!(
        text.contains("already published; a rebuild is a new version"),
        "{text}"
    );
    assert_eq!(index.packages.len(), 1);
}

#[test]
fn insert_refuses_a_package_the_index_could_not_be_read_back_with() {
    let mut index = Index::empty();
    let mut p = package("gcce;12.1.0");
    p.archives[0].sha256 = "ABC".into();
    let e = index.insert(p).unwrap_err().to_string();
    assert!(e.contains("sha256 `ABC`"), "{e}");
    assert!(index.packages.is_empty());
}

#[test]
fn prints_the_layout_of_the_spec() {
    let index = Index::parse(&spec_example(), "public").unwrap();
    let text = index.to_toml().unwrap();
    assert!(text.starts_with("schema = 1\n"), "{text}");
    assert!(
        text.contains("[[package]]\nid = \"gcce;12.1.0\"\n"),
        "{text}"
    );
    assert!(text.contains("depends = []\n"), "{text}");
    assert!(
        text.contains("[[package.archive]]\nhost = \"any\"\n"),
        "{text}"
    );
}
