//! Several homes (`SYMDEV_HOME`s) share one download cache (`XDG_CACHE_HOME`): parallel
//! installs must neither write into one another's download nor take away the archive
//! another one is extracting.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use super::World;
use crate::{Fetch, FileFetch, Result, SdkError, SdkHome};

/// A `file://` fetch that writes slowly, so parallel downloads overlap.
struct Slow {
    downloads: AtomicUsize,
}

impl Fetch for Slow {
    fn text(&self, url: &str) -> Result<String> {
        FileFetch.text(url)
    }

    fn download(&self, url: &str, dest: &Path, _limit: u64) -> Result<u64> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        let path = url.trim_start_matches("file://");
        let bytes = fs::read(path).map_err(|e| SdkError::Other(e.to_string()))?;
        let mut file = File::create(dest).map_err(|e| SdkError::Other(e.to_string()))?;
        for chunk in bytes.chunks(16) {
            file.write_all(chunk)
                .map_err(|e| SdkError::Other(e.to_string()))?;
            thread::sleep(Duration::from_millis(2));
        }
        Ok(bytes.len() as u64)
    }
}

#[test]
fn homes_sharing_one_cache_install_in_parallel_with_one_download() {
    const HOMES: usize = 8;
    let w = World::new();
    let fetch = Slow {
        downloads: AtomicUsize::new(0),
    };
    let homes: Vec<_> = (0..HOMES)
        .map(|n| SdkHome::new(w.tmp.path().join(format!("home-{n}")), w.cache()))
        .collect();
    let start = Barrier::new(HOMES);
    let results: Vec<_> = thread::scope(|s| {
        let threads: Vec<_> = homes
            .iter()
            .map(|home| {
                let (start, fetch, w) = (&start, &fetch, &w);
                s.spawn(move || {
                    start.wait();
                    home.install(&w.id, &w.source, fetch, &w.entry, || {})
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    for (n, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "home {n}: {result:?}");
        let gxx = homes[n]
            .package_dir(&w.id)
            .join("bin/arm-none-symbianelf-g++");
        assert_eq!(fs::read(gxx).unwrap(), b"gxx", "home {n}");
    }
    assert_eq!(fetch.downloads.load(Ordering::SeqCst), 1);
    let parts: Vec<_> = fs::read_dir(w.cache())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".part"))
        .collect();
    assert!(parts.is_empty(), "{parts:?}");
}

/// An interrupted download (Ctrl-C) leaves its `.part`; the next download of that archive,
/// which holds the cache's lock, removes it.
#[test]
fn a_stale_part_of_the_archive_is_removed_by_the_next_download() {
    let w = World::new();
    fs::create_dir_all(w.cache()).unwrap();
    let stale = w
        .cache()
        .join(format!("{}.tar.gz.1-0.part", w.entry.sha256));
    let other = w
        .cache()
        .join(format!("{}.tar.gz.1-0.part", "f".repeat(64)));
    fs::write(&stale, b"half").unwrap();
    fs::write(&other, b"another archive").unwrap();
    w.home()
        .install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    assert!(!stale.exists());
    assert!(
        other.exists(),
        "only parts of the archive being fetched are touched"
    );
}
