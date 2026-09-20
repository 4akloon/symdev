//! `Elf2E32Tool`: argv for the elf2e32 binary.
use std::path::{Path, PathBuf};

use crate::elf2e32::Elf2E32;

pub struct Elf2E32Tool {
    pub elf2e32: PathBuf,
}

impl Elf2E32Tool {
    pub fn new(elf2e32: &Path) -> Self {
        Self {
            elf2e32: elf2e32.to_path_buf(),
        }
    }

    pub fn args(&self, img: &Elf2E32) -> Vec<String> {
        let mut args = vec![
            self.elf2e32.display().to_string(),
            format!("--uid1={:#010x}", img.uid1),
            format!("--uid3={:#010x}", img.uid3),
        ];
        if let Some(cap) = &img.capability {
            args.push(format!("--capability={cap}"));
        }
        args.extend([
            format!("--fpu={}", img.fpu),
            format!("--targettype={}", img.targettype),
            format!("--output={}", img.output.display()),
            format!("--elfinput={}", img.elfinput.display()),
            format!("--linkas={}", img.linkas),
            format!("--libpath={}", img.libpath.display()),
        ]);
        if img.uncompressed {
            args.push("--uncompressed".into());
        }
        args
    }
}
