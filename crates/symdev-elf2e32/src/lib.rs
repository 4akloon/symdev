use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

mod deflate;
mod e32;
mod elf;

pub use deflate::E32Deflate;
pub use e32::E32Layout;
pub use e32::E32RelocSection;
pub use e32::E32Uid;
pub use e32::{E32CodeSection, E32Ordinals};
pub use e32::{E32Image, E32Time};
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
    /// Observed `--uncompressed` (`elf2e32 --help`; experiment 44).
    pub uncompressed: bool,
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
        let mut uncompressed = false;
        for tok in tokens {
            if tok == "--uncompressed" {
                uncompressed = true;
                continue;
            }
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
            uncompressed,
        })
    }

    pub fn uid(&self) -> E32Uid {
        E32Uid::for_exe(self.uid1, self.uid3)
    }

    /// Ordinals for every import of `elf`, read from the DSOs `.gnu.version_r` names
    /// under `--libpath`.
    pub fn ordinals(&self, elf: &ElfImage) -> Result<E32Ordinals> {
        let mut ordinals = E32Ordinals::default();
        let mut dsos = std::collections::BTreeMap::new();
        for imp in elf.import_relocs()? {
            if !dsos.contains_key(&imp.dso) {
                let path = self.libpath.join(&imp.dso);
                let bytes = std::fs::read(&path)
                    .map_err(|e| Error::Other(format!("read DSO {path:?}: {e}")))?;
                dsos.insert(imp.dso.clone(), ElfImage::parse(bytes)?);
            }
            let dso = &dsos[&imp.dso];
            let ordinal = dso.dso_ordinal(&imp.symbol)?.ok_or_else(|| {
                Error::Other(format!("{} does not export {}", imp.dso, imp.symbol))
            })?;
            ordinals.insert(imp.dll, imp.symbol, ordinal);
        }
        Ok(ordinals)
    }

    /// Reads `--elfinput` and the `--libpath` DSOs, stamps the current time.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let bytes = std::fs::read(&self.elfinput)
            .map_err(|e| Error::Other(format!("read ELF {:?}: {e}", self.elfinput)))?;
        let elf = ElfImage::parse(bytes)?;
        let ordinals = self.ordinals(&elf)?;
        self.encode_elf(
            &elf,
            &ordinals,
            E32Time::from_system(std::time::SystemTime::now())?,
        )
    }

    /// E32 bytes for an already-read ELF (no host I/O).
    pub fn encode_elf(
        &self,
        elf: &ElfImage,
        ordinals: &E32Ordinals,
        time: E32Time,
    ) -> Result<Vec<u8>> {
        if self.targettype != "EXE" {
            return Err(Error::Other(format!(
                "TODO: native elf2e32 --targettype={} (only EXE observed)",
                self.targettype
            )));
        }
        if self.fpu != "softvfp" {
            return Err(Error::Other(format!(
                "TODO: native elf2e32 --fpu={} (only softvfp observed)",
                self.fpu
            )));
        }
        let names: Vec<&str> = self
            .capability
            .as_deref()
            .map(|c| c.split('+').collect())
            .unwrap_or_default();
        let caps = symdev_core::Capabilities::from_names(&names)?;
        let image = E32Image::exe(elf, self.uid(), caps, ordinals, time)?;
        if self.uncompressed {
            Ok(image.uncompressed())
        } else {
            image.compressed()
        }
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
        if img.uncompressed {
            args.push("--uncompressed".into());
        }
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

    fn hello_elf() -> ElfImage {
        let hex = include_str!("testdata/hello.elf.hex");
        let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        ElfImage::parse(bytes).unwrap()
    }

    fn hello_ordinals() -> E32Ordinals {
        let mut ordinals = E32Ordinals::default();
        for line in include_str!("testdata/hello_ordinals.txt").lines() {
            if line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            let ordinal = u32::from_str_radix(f[2].trim_start_matches("0x"), 16).unwrap();
            ordinals.insert(f[0], f[1], ordinal);
        }
        ordinals
    }

    fn experiment_44() -> Elf2E32 {
        let mut a = args(&[
            "elf2e32",
            "--uid1=0x1000007a",
            "--uid3=0xe79e4cf9",
            "--capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location",
            "--fpu=softvfp",
            "--targettype=EXE",
            "--output=hello_u.exe",
            "--elfinput=hello.elf",
            "--linkas=hello{000a0000}[e79e4cf9].exe",
            "--libpath=/sdk/epoc32/release/armv5/lib",
        ]);
        a.push("--uncompressed".into());
        Elf2E32::from_args(&a).unwrap()
    }

    #[test]
    fn experiment_44_encode_matches_uncompressed_golden() {
        let hex = include_str!("testdata/hello_uncompressed.exe.hex");
        let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        let golden: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let job = experiment_44();
        assert!(job.uncompressed);
        let bytes = job
            .encode_elf(
                &hello_elf(),
                &hello_ordinals(),
                E32Time(0x00e3_3986_c0a8_ae80),
            )
            .unwrap();
        assert_eq!(bytes, golden);
    }

    #[test]
    fn experiment_6_encode_matches_frozen_hello_exe() {
        // Frozen experiment-6 hello.exe header time.
        let bytes = experiment_6()
            .encode_elf(
                &hello_elf(),
                &hello_ordinals(),
                E32Time(0x00e3_3963_208d_5e00),
            )
            .unwrap();
        assert_eq!(bytes.len(), 3588);
        assert_eq!(bytes, hello_exe());
    }

    #[test]
    fn experiment_44_tool_args_end_with_uncompressed() {
        let args = Elf2E32Tool::new(Path::new("/elf2e32")).args(&experiment_44());
        assert_eq!(args.last().map(String::as_str), Some("--uncompressed"));
    }

    #[test]
    fn fp2_dso_ordinals_match_experiment_44_table() {
        // Reads SDK DSOs only when SYMDEV_EPOCROOT points at an FP2 SDK; skipped otherwise.
        let Some(root) = std::env::var_os("SYMDEV_EPOCROOT") else {
            return;
        };
        let libpath = PathBuf::from(root).join("epoc32/release/armv5/lib");
        if !libpath.is_dir() {
            return;
        }
        let job = Elf2E32 {
            libpath,
            ..experiment_6()
        };
        let ordinals = job.ordinals(&hello_elf()).unwrap();
        for line in include_str!("testdata/hello_ordinals.txt").lines() {
            if line.starts_with('#') {
                continue;
            }
            let mut f = line.split_whitespace();
            let (dll, symbol, want) = (f.next().unwrap(), f.next().unwrap(), f.next().unwrap());
            let want = u32::from_str_radix(want.trim_start_matches("0x"), 16).unwrap();
            assert_eq!(ordinals.get(dll, symbol), Some(want), "{dll} {symbol}");
        }
    }

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
