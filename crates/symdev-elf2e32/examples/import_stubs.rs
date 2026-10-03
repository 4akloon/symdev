//! Experiment 112 driver: `import_stubs <first-link.elf> <stubs.o>` writes the 8-byte import
//! stubs for the functions an lld link calls through its PLT and prints one function per
//! line (each needs `--wrap=<f>` on the second link).
use symdev_elf2e32::{ElfImage, ImportStubs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [elf, object] = args.as_slice() else {
        return Err("usage: import_stubs <first-link.elf> <stubs.o>".into());
    };
    let stubs = ImportStubs::from_first_link(&ElfImage::parse(std::fs::read(elf)?)?)?;
    std::fs::write(object, stubs.object())?;
    for f in stubs.functions() {
        println!("{f}");
    }
    Ok(())
}
