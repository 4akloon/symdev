use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::SdkHome;
use crate::{ArchiveEntry, Auth, Fetch, FileFetch, Host, PackageId, ReproducibleTarGz};
use crate::{Result, SdkError, SourceSpec};

mod placement;
mod receipts;

/// Counts downloads, so a test can tell a cache hit from a fetch.
struct Counting {
    downloads: AtomicUsize,
}

impl Fetch for Counting {
    fn text(&self, url: &str) -> Result<String> {
        FileFetch.text(url)
    }

    fn download(&self, url: &str, dest: &Path) -> Result<u64> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        FileFetch.download(url, dest)
    }
}

fn counting() -> Counting {
    Counting {
        downloads: AtomicUsize::new(0),
    }
}

/// A temp dir with a `file://` source holding one packed `gcce;12.1.0`.
struct World {
    tmp: tempfile::TempDir,
    source: SourceSpec,
    entry: ArchiveEntry,
    id: PackageId,
}

impl World {
    fn new() -> World {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("tree");
        fs::create_dir_all(tree.join("bin")).unwrap();
        fs::write(tree.join("bin/arm-none-symbianelf-g++"), b"gxx").unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join("gcce/12.1.0")).unwrap();
        let packed = repo.join("packed.tar.gz");
        let (sha256, size) = ReproducibleTarGz::pack(&tree, &["."], &packed).unwrap();
        let url = format!("gcce/12.1.0/{sha256}.tar.gz");
        fs::rename(&packed, repo.join(&url)).unwrap();
        let base = format!("file://{}", repo.display());
        World {
            source: SourceSpec::new("local", &base, Auth::None).unwrap(),
            entry: ArchiveEntry {
                host: Host::Any,
                url,
                sha256,
                size,
            },
            id: PackageId::parse("gcce;12.1.0").unwrap(),
            tmp,
        }
    }

    fn home(&self) -> SdkHome {
        SdkHome::new(self.tmp.path().join("home"), self.cache())
    }

    fn cache(&self) -> PathBuf {
        self.tmp.path().join("cache")
    }

    fn cached(&self) -> PathBuf {
        self.cache().join(format!("{}.tar.gz", self.entry.sha256))
    }
}

#[test]
fn installs_the_files_and_writes_the_receipt() {
    let w = World::new();
    let home = w.home();
    assert_eq!(home.installed(&w.id).unwrap(), None);
    let receipt = home
        .install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let dir = home.package_dir(&w.id);
    assert_eq!(dir, w.tmp.path().join("home/gcce/12.1.0"));
    assert_eq!(
        fs::read(dir.join("bin/arm-none-symbianelf-g++")).unwrap(),
        b"gxx"
    );
    assert_eq!(receipt.id, w.id);
    assert_eq!(receipt.sha256, w.entry.sha256);
    assert_eq!(receipt.source, "local");
    assert_eq!(receipt.url, format!("{}{}", w.source.base, w.entry.url));
    assert_eq!(home.installed(&w.id).unwrap(), Some(receipt));
}

#[test]
fn a_second_install_downloads_nothing() {
    let w = World::new();
    let fetch = counting();
    w.home()
        .install(&w.id, &w.source, &fetch, &w.entry, || {})
        .unwrap();
    w.home()
        .install(&w.id, &w.source, &fetch, &w.entry, || {})
        .unwrap();
    assert_eq!(fetch.downloads.load(Ordering::SeqCst), 1);
}

#[test]
fn a_verified_cached_archive_is_reused() {
    let w = World::new();
    let fetch = counting();
    w.home()
        .install(&w.id, &w.source, &fetch, &w.entry, || {})
        .unwrap();
    assert!(w.home().uninstall(&w.id).unwrap());
    w.home()
        .install(&w.id, &w.source, &fetch, &w.entry, || {})
        .unwrap();
    assert_eq!(fetch.downloads.load(Ordering::SeqCst), 1);
}

#[test]
fn a_package_dir_without_a_receipt_is_installed_again() {
    let w = World::new();
    let dir = w.home().package_dir(&w.id);
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::write(dir.join("bin/half-written"), b"x").unwrap();
    w.home()
        .install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    assert!(!dir.join("bin/half-written").exists());
    assert!(dir.join("bin/arm-none-symbianelf-g++").exists());
}

#[test]
fn a_truncated_cached_download_is_fetched_again_not_extracted() {
    let w = World::new();
    let whole =
        fs::read(w.source.base.trim_start_matches("file://").to_string() + &w.entry.url).unwrap();
    fs::create_dir_all(w.cache()).unwrap();
    fs::write(w.cached(), &whole[..whole.len() / 2]).unwrap();
    let fetch = counting();
    w.home()
        .install(&w.id, &w.source, &fetch, &w.entry, || {})
        .unwrap();
    assert_eq!(fetch.downloads.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(w.cached()).unwrap(), whole);
}

#[test]
fn a_hash_mismatch_deletes_the_download_and_installs_nothing() {
    let w = World::new();
    let mut entry = w.entry.clone();
    entry.sha256 = "0".repeat(64);
    match w
        .home()
        .install(&w.id, &w.source, &FileFetch, &entry, || {})
    {
        Err(SdkError::HashMismatch {
            id,
            url,
            expected,
            actual,
            ..
        }) => {
            assert_eq!(id, "gcce;12.1.0");
            assert!(url.ends_with(&w.entry.url));
            assert_eq!(expected, entry.sha256);
            assert_eq!(actual, w.entry.sha256);
        }
        other => panic!("expected HashMismatch, got {other:?}"),
    }
    let left: Vec<_> = fs::read_dir(w.cache()).unwrap().collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(!w.home().package_dir(&w.id).exists());
}

#[test]
fn a_sha256_that_is_not_hex_never_becomes_a_cache_path() {
    let w = World::new();
    let mut entry = w.entry.clone();
    entry.sha256 = "../../escape".into();
    let e = w
        .home()
        .install(&w.id, &w.source, &FileFetch, &entry, || {})
        .unwrap_err();
    assert!(e.to_string().contains("../../escape"), "{e}");
    assert!(!w.tmp.path().join("escape.tar.gz").exists());
}

#[test]
fn an_unsafe_archive_leaves_no_staging_and_no_package() {
    let w = World::new();
    let mut entry = w.entry.clone();
    let repo = PathBuf::from(w.source.base.trim_start_matches("file://"));
    let bad = repo.join("bad.tar.gz");
    let gz = flate2::write::GzEncoder::new(fs::File::create(&bad).unwrap(), Default::default());
    let mut tar = tar::Builder::new(gz);
    let mut h = tar::Header::new_gnu();
    h.as_old_mut().name[..9].copy_from_slice(b"../escape");
    h.set_size(1);
    h.set_cksum();
    tar.append(&h, &b"x"[..]).unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let bytes = fs::read(&bad).unwrap();
    entry.url = "bad.tar.gz".into();
    entry.size = bytes.len() as u64;
    entry.sha256 = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
    let e = w
        .home()
        .install(&w.id, &w.source, &FileFetch, &entry, || {})
        .unwrap_err();
    assert!(matches!(e, SdkError::UnsafeEntry { .. }), "{e:?}");
    let staging = w.home().root().join(".staging");
    assert_eq!(fs::read_dir(&staging).unwrap().count(), 0);
    assert!(!w.home().package_dir(&w.id).exists());
    assert!(!w.tmp.path().join("home/escape").exists());
}

#[test]
fn a_stale_staging_dir_is_removed_under_the_lock() {
    let w = World::new();
    let stale = w.home().root().join(".staging/1-0");
    fs::create_dir_all(&stale).unwrap();
    w.home()
        .install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    assert!(!stale.exists());
}

#[test]
fn two_threads_installing_one_id_end_with_one_install() {
    let w = World::new();
    let fetch = counting();
    let [a, b] = std::thread::scope(|s| {
        let a = s.spawn(|| w.home().install(&w.id, &w.source, &fetch, &w.entry, || {}));
        let b = s.spawn(|| w.home().install(&w.id, &w.source, &fetch, &w.entry, || {}));
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(a.unwrap(), b.unwrap());
    assert_eq!(fetch.downloads.load(Ordering::SeqCst), 1);
    let gcce = fs::read_dir(w.home().root().join("gcce")).unwrap().count();
    assert_eq!(gcce, 1);
}

#[test]
fn starting_runs_only_when_the_package_is_really_installed() {
    let w = World::new();
    let mut starts = 0;
    w.home()
        .install(&w.id, &w.source, &FileFetch, &w.entry, || starts += 1)
        .unwrap();
    // As when a parallel build installed it between the caller's check and the lock.
    w.home()
        .install(&w.id, &w.source, &FileFetch, &w.entry, || starts += 1)
        .unwrap();
    assert_eq!(starts, 1);
}
