use std::process::ExitCode;

use symdev_sis::Makesis;

fn main() -> ExitCode {
    match Makesis::from_args(&std::env::args().collect::<Vec<_>>()).and_then(|m| m.run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
