use std::process::ExitCode;

use symdev_makekeys::Makekeys;

fn main() -> ExitCode {
    match Makekeys::from_args(&std::env::args().collect::<Vec<_>>()).and_then(|m| m.run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
