use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

mod e32;
mod elf;

pub use e32::E32Layout;
pub use e32::E32RelocSection;
pub use e32::E32Uid;
pub use e32::{E32ImageHeader, E32ImageHeaderJ, E32ImageHeaderV};
pub use e32::{E32ImportBlock, E32ImportSection};
pub use elf::ElfImportReloc;
pub use elf::ElfLocalReloc;
pub use elf::{ElfImage, ElfSegment};

pub struct Elf2E32 {
    pub uid1: u32,
    pub uid3: u32,
    pub capability: Option<String>,
    pub fpu: String,
    pub targettype: String,
    pub output: PathBuf,
    pub elfinput: PathBuf,
    pub linkas: String,
    pub libpath: PathBuf,
}

impl Elf2E32 {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let tokens = match args.first().map(String::as_str) {
            Some(s) if !s.starts_with("--") => &args[1..],
            _ => args,
        };
        let mut uid1 = None;
        let mut uid3 = None;
        let mut capability = None;
        let mut fpu = None;
        let mut targettype = None;
        let mut output = None;
        let mut elfinput = None;
        let mut linkas = None;
        let mut libpath = None;
        for tok in tokens {
            let Some((key, val)) = tok.split_once('=') else {
                return Err(Error::Other(format!("unknown elf2e32 arg: {tok}")));
            };
            match key {
                "--uid1" => uid1 = Some(parse_uid(val)?),
                "--uid3" => uid3 = Some(parse_uid(val)?),
                "--capability" => capability = Some(val.to_string()),
                "--fpu" => fpu = Some(val.to_string()),
                "--targettype" => targettype = Some(val.to_string()),
                "--output" => output = Some(PathBuf::from(val)),
                "--elfinput" => elfinput = Some(PathBuf::from(val)),
                "--linkas" => linkas = Some(val.to_string()),
                "--libpath" => libpath = Some(PathBuf::from(val)),
                _ => return Err(Error::Other(format!("unknown elf2e32 flag: {key}"))),
            }
        }
        Ok(Self {
            uid1: uid1.ok_or_else(|| Error::Other("elf2e32 missing --uid1".into()))?,
            uid3: uid3.ok_or_else(|| Error::Other("elf2e32 missing --uid3".into()))?,
            capability,
            fpu: fpu.ok_or_else(|| Error::Other("elf2e32 missing --fpu".into()))?,
            targettype: targettype
                .ok_or_else(|| Error::Other("elf2e32 missing --targettype".into()))?,
            output: output.ok_or_else(|| Error::Other("elf2e32 missing --output".into()))?,
            elfinput: elfinput.ok_or_else(|| Error::Other("elf2e32 missing --elfinput".into()))?,
            linkas: linkas.ok_or_else(|| Error::Other("elf2e32 missing --linkas".into()))?,
            libpath: libpath.ok_or_else(|| Error::Other("elf2e32 missing --libpath".into()))?,
        })
    }

    pub fn uid(&self) -> E32Uid {
        E32Uid::for_exe(self.uid1, self.uid3)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let _ = self;
        Err(Error::Other("TODO: native ELF→E32 encode".into()))
    }
}

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
        args
    }
}

fn parse_uid(s: &str) -> Result<u32> {
    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(hex, 16).map_err(|_| Error::Other(format!("invalid UID: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| (*t).to_string()).collect()
    }

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_exe() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello.exe.hex"))
    }

    fn experiment_6() -> Elf2E32 {
        Elf2E32::from_args(&args(&[
            "elf2e32",
            "--uid1=0x1000007a",
            "--uid3=0xe79e4cf9",
            "--capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location",
            "--fpu=softvfp",
            "--targettype=EXE",
            "--output=hello.exe",
            "--elfinput=hello.elf",
            "--linkas=hello{000a0000}[e79e4cf9].exe",
            "--libpath=/sdk/epoc32/release/armv5/lib",
        ]))
        .unwrap()
    }

    #[test]
    fn hello_exe_uid_bytes_match_experiment_6() {
        let golden = hello_exe();
        assert_eq!(golden.len(), 3588);
        let want = [
            0x7a, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0xb0, 0x08,
            0x32, 0xc1,
        ];
        assert_eq!(E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9).bytes(), want);
        assert_eq!(&golden[..16], &want);
    }

    #[test]
    fn hello_exe_uid_checked_matches_experiment_6() {
        assert_eq!(
            E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9).crc().checked(),
            0xc132_08b0
        );
    }

    #[test]
    fn experiment_6_job_uid_matches_hello_exe_prefix() {
        let golden = hello_exe();
        assert_eq!(experiment_6().uid().bytes(), golden[..16]);
    }

    #[test]
    fn from_args_match_experiment_6() {
        let job = experiment_6();
        assert_eq!(job.uid1, 0x1000_007a);
        assert_eq!(job.uid3, 0xe79e_4cf9);
        assert_eq!(
            job.capability.as_deref(),
            Some(
                "LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location"
            )
        );
        assert_eq!(job.fpu, "softvfp");
        assert_eq!(job.targettype, "EXE");
        assert_eq!(job.output, PathBuf::from("hello.exe"));
        assert_eq!(job.elfinput, PathBuf::from("hello.elf"));
        assert_eq!(job.linkas, "hello{000a0000}[e79e4cf9].exe");
        assert_eq!(job.libpath, PathBuf::from("/sdk/epoc32/release/armv5/lib"));
    }

    #[test]
    fn from_args_omits_empty_capability() {
        let job = Elf2E32::from_args(&args(&[
            "elf2e32",
            "--uid1=0x1000007a",
            "--uid3=0xe79e4cf9",
            "--fpu=softvfp",
            "--targettype=EXE",
            "--output=hello.exe",
            "--elfinput=hello.elf",
            "--linkas=hello{000a0000}[e79e4cf9].exe",
            "--libpath=/sdk/epoc32/release/armv5/lib",
        ]))
        .unwrap();
        assert_eq!(job.capability, None);
    }
}
