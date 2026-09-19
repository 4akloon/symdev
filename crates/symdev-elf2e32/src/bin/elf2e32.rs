use std::process::ExitCode;

use symdev_elf2e32::Elf2E32;

fn main() -> ExitCode {
    match Elf2E32::from_args(&std::env::args().collect::<Vec<_>>())
        .and_then(|job| job.write_outputs())
    {
        Ok(exports) => {
            for name in exports.iter().flat_map(|e| e.new_names()) {
                eprintln!("warning: new export {name} is not yet frozen");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
