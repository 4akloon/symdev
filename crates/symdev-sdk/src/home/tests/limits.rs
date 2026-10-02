//! The index's `size` bounds a download: a longer file stops it there.

use std::fs;

use super::World;
use crate::{FileFetch, SdkError};

#[test]
fn a_file_longer_than_the_index_says_stops_the_download() {
    let w = World::new();
    let mut entry = w.entry.clone();
    entry.size -= 1;
    match w
        .home()
        .install(&w.id, &w.source, &FileFetch, &entry, || {})
    {
        Err(SdkError::Fetch { url, detail }) => {
            assert!(url.ends_with(&entry.url), "{url}");
            let expected = format!("more than the {} bytes", entry.size);
            assert!(detail.contains(&expected), "{detail}");
        }
        other => panic!("expected the download to stop, got {other:?}"),
    }
    let parts = fs::read_dir(w.cache()).unwrap().count();
    assert_eq!(parts, 1, "only the cache's lock is left");
}
