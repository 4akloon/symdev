//! `Interrupt`: Ctrl+C, which stops the app and not the emulator (spec §6.5). cargo `exec`s
//! the runner for `cargo run`, so the terminal's SIGINT reaches this process; the emulator
//! runs in a session of its own and never gets it.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use symdev_core::{Error, Result};

pub(crate) struct Interrupt(Arc<AtomicBool>);

impl Interrupt {
    pub fn install() -> Result<Self> {
        let flag = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&flag);
        ctrlc::set_handler(move || seen.store(true, Ordering::SeqCst))
            .map_err(|e| Error::Other(format!("cannot catch Ctrl+C: {e}")))?;
        Ok(Self(flag))
    }

    pub fn raised(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
