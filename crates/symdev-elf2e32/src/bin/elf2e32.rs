use std::process::ExitCode;

use symdev_elf2e32::Elf2E32;

fn main() -> ExitCode {
    match Elf2E32::from_args(&std::env::args().collect::<Vec<_>>()) {
        Ok(_job) => todo!("native ELF→E32 encode"),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
