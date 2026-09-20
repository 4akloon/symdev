use std::process::ExitCode;

use symdev_rcomp::Rcomp;

fn main() -> ExitCode {
    match Rcomp::from_args(&std::env::args().collect::<Vec<_>>()).and_then(|job| job.run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
