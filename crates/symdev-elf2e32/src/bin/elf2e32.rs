use std::process::ExitCode;

use symdev_elf2e32::Elf2E32;

fn main() -> ExitCode {
    match Elf2E32::from_args(&std::env::args().collect::<Vec<_>>()) {
        Ok(job) => match job.encode().and_then(|bytes| {
            std::fs::write(&job.output, bytes)
                .map_err(|e| symdev_core::Error::Other(format!("write {:?}: {e}", job.output)))
        }) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
