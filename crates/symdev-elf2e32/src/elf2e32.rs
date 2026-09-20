//! `Elf2E32`: the elf2e32 job (parsed args, ELF-to-E32 encoding).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::{
    E32DefFile, E32Dll, E32Dso, E32Exports, E32Image, E32Ordinals, E32Target, E32Time, E32Uid,
    ElfImage,
};

mod parse_uid;
use parse_uid::parse_uid;

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
    /// DLL options from the SDK GCCE recipe (`cl_bpabi.pm`, experiment 52).
    pub uid2: Option<u32>,
    pub sid: Option<u32>,
    pub dso: Option<PathBuf>,
    pub defoutput: Option<PathBuf>,
    /// Frozen exports (`.def`, experiment 54).
    pub definput: Option<PathBuf>,
    /// Accepted from the SDK recipe; no effect observed (experiment 54: elf2e32_next
    /// writes the same exports with and without it).
    pub ignorenoncallable: bool,
    /// `--dlldata`: allow writable static data in a DLL (`cl_bpabi.pm`, MMP
    /// `EPOCALLOWDLLDATA`).
    pub dlldata: bool,
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
        let mut ignorenoncallable = false;
        let mut dlldata = false;
        let (mut uid2, mut sid, mut dso, mut defoutput, mut definput) =
            (None, None, None, None, None);
        for tok in tokens {
            if tok == "--uncompressed" {
                uncompressed = true;
                continue;
            }
            if tok == "--ignorenoncallable" {
                ignorenoncallable = true;
                continue;
            }
            if tok == "--dlldata" {
                dlldata = true;
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
                "--uid2" => uid2 = Some(parse_uid(val)?),
                "--sid" => sid = Some(parse_uid(val)?),
                "--dso" => dso = Some(PathBuf::from(val)),
                "--defoutput" => defoutput = Some(PathBuf::from(val)),
                "--definput" => definput = Some(PathBuf::from(val)),
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
            uid2,
            sid,
            dso,
            defoutput,
            definput,
            ignorenoncallable,
            dlldata,
        })
    }

    pub fn uid(&self) -> E32Uid {
        match self.uid2 {
            Some(uid2) => E32Uid {
                uid1: self.uid1,
                uid2,
                uid3: self.uid3,
            },
            None => E32Uid::for_exe(self.uid1, self.uid3),
        }
    }

    fn target(&self) -> Result<E32Target> {
        match self.targettype.as_str() {
            "EXE" => Ok(E32Target::Exe),
            "DLL" => Ok(E32Target::Dll),
            other => Err(Error::Other(format!(
                "TODO: native elf2e32 --targettype={other} (EXE and DLL observed)"
            ))),
        }
    }

    /// Ordinals for every import of `elf`, read from the DSOs `.gnu.version_r` names
    /// under `--libpath`.
    pub fn ordinals(&self, elf: &ElfImage) -> Result<E32Ordinals> {
        let mut ordinals = E32Ordinals::default();
        let mut dsos = std::collections::BTreeMap::new();
        for imp in elf.import_relocs()? {
            if !dsos.contains_key(&imp.dso) {
                let path = self.find_dso(&imp.dso)?;
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
        let elf = self.read_elf()?;
        let ordinals = self.ordinals(&elf)?;
        self.encode_elf(
            &elf,
            &ordinals,
            E32Time::from_system(std::time::SystemTime::now())?,
        )
    }

    /// Write `--output`, and for a DLL `--defoutput` and `--dso`. Returns the DLL's
    /// exports so a caller can report the ones not yet frozen.
    pub fn write_outputs(&self) -> Result<Option<E32Exports>> {
        let elf = self.read_elf()?;
        let ordinals = self.ordinals(&elf)?;
        let exports = self.exports(&elf)?;
        let image = self.encode_elf_with(
            &elf,
            &ordinals,
            E32Time::from_system(std::time::SystemTime::now())?,
            exports.as_ref(),
        )?;
        write(&self.output, &image)?;
        if let Some(exports) = &exports {
            if let Some(def) = &self.defoutput {
                write(def, exports.def_text().as_bytes())?;
            }
            if let Some(dso) = &self.dso {
                let soname = dso
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| Error::Other(format!("bad --dso path {dso:?}")))?;
                let bytes = E32Dso {
                    soname,
                    linkas: &self.linkas,
                    exports,
                }
                .bytes()?;
                write(dso, &bytes)?;
            }
        }
        Ok(exports)
    }

    /// A DLL's exports, ordinals frozen by `--definput` when given; `None` for an EXE.
    pub fn exports(&self, elf: &ElfImage) -> Result<Option<E32Exports>> {
        if self.target()? != E32Target::Dll {
            if self.definput.is_some() {
                return Err(Error::Other(
                    "TODO: --definput for an EXE (not observed)".into(),
                ));
            }
            return Ok(None);
        }
        let frozen = match &self.definput {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| Error::Other(format!("read --definput {path:?}: {e}")))?;
                Some(E32DefFile::parse(&text)?)
            }
            None => None,
        };
        E32Exports::from_elf(elf, frozen.as_ref()).map(Some)
    }

    fn find_dso(&self, dso: &str) -> Result<PathBuf> {
        crate::LibPath::parse(&self.libpath.to_string_lossy()).find(dso)
    }

    fn read_elf(&self) -> Result<ElfImage> {
        let bytes = std::fs::read(&self.elfinput)
            .map_err(|e| Error::Other(format!("read ELF {:?}: {e}", self.elfinput)))?;
        ElfImage::parse(bytes)
    }

    /// E32 bytes for an already-read ELF (reads only `--definput`).
    pub fn encode_elf(
        &self,
        elf: &ElfImage,
        ordinals: &E32Ordinals,
        time: E32Time,
    ) -> Result<Vec<u8>> {
        let exports = self.exports(elf)?;
        self.encode_elf_with(elf, ordinals, time, exports.as_ref())
    }

    /// E32 bytes for an already-read ELF and its exports (no host I/O).
    pub fn encode_elf_with(
        &self,
        elf: &ElfImage,
        ordinals: &E32Ordinals,
        time: E32Time,
        exports: Option<&E32Exports>,
    ) -> Result<Vec<u8>> {
        if self.sid.is_some_and(|sid| sid != self.uid3) {
            return Err(Error::Other(
                "TODO: --sid other than --uid3 (not observed)".into(),
            ));
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
        let dll = exports.map(|exports| E32Dll {
            exports,
            allow_data: self.dlldata,
        });
        let image = E32Image::new(elf, self.uid(), caps, ordinals, time, dll)?;
        if self.uncompressed {
            Ok(image.uncompressed())
        } else {
            image.compressed()
        }
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).map_err(|e| Error::Other(format!("write {path:?}: {e}")))
}

#[cfg(test)]
mod tests;
