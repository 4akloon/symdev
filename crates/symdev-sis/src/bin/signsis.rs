use std::process::ExitCode;

use symdev_sis::Signsis;

fn main() -> ExitCode {
    match Signsis::from_args(&std::env::args().collect::<Vec<_>>()) {
        Ok(_job) => todo!("inflate unsigned SIS and attach signatures"),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
