//! `LogTail`: the guest's `RDebug` lines from the emulator's log as they are written. An
//! `RDebug::Print` is logged as `T <source>:<line> [Emulated.Stdout]: <text>` (experiment 114
//! §2); the text after the marker is what the app printed.
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const MARKER: &str = "[Emulated.Stdout]: ";

pub(crate) struct LogTail {
    path: PathBuf,
    offset: u64,
    inode: u64,
    partial: Vec<u8>,
}

impl LogTail {
    /// From the log's end now: what was there before is not this run's.
    pub fn from_end(path: &Path) -> Self {
        let meta = std::fs::metadata(path).ok();
        Self {
            path: path.to_path_buf(),
            offset: meta.as_ref().map_or(0, |m| m.len()),
            inode: meta.as_ref().map_or(0, |m| m.ino()),
            partial: Vec::new(),
        }
    }

    /// The guest lines completed since the last poll. A log replaced (a new file) or cut
    /// shorter is read from its start. A log that cannot be read gives nothing.
    pub fn poll(&mut self) -> Vec<String> {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        let Ok(meta) = file.metadata() else {
            return Vec::new();
        };
        if meta.ino() != self.inode || meta.len() < self.offset {
            self.offset = 0;
            self.inode = meta.ino();
            self.partial.clear();
        }
        let mut bytes = Vec::new();
        if file.seek(SeekFrom::Start(self.offset)).is_err() || file.read_to_end(&mut bytes).is_err()
        {
            return Vec::new();
        }
        self.offset += bytes.len() as u64;
        self.partial.extend_from_slice(&bytes);
        let Some(end) = self.partial.iter().rposition(|b| *b == b'\n') else {
            return Vec::new();
        };
        let complete: Vec<u8> = self.partial.drain(..=end).collect();
        String::from_utf8_lossy(&complete)
            .lines()
            .filter_map(|l| l.split_once(MARKER).map(|(_, text)| text.to_string()))
            .collect()
    }
}
