use std::process::ExitCode;

use symdev_rcomp::Rcomp;

fn main() -> ExitCode {
    match Rcomp::from_args(&std::env::args().collect::<Vec<_>>()) {
        Ok(_job) => todo!("RSS source parse / .rsg"),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
