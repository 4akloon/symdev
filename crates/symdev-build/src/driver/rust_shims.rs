//! `RustBuild`: compiling the SDK's C++ shim and archiving it beside the Rust one.
//!
//! The application names none of this. `shims/common` goes into every Rust program so
//! that a leaving Symbian call is `TRAP`ped before it can reach a Rust frame (design
//! spec §7, step 70); `shims/s60` — the Avkon subclasses — goes in only when the
//! manifest has a `[ui]` section, because it brings five more import libraries and an
//! `E32Main` of its own that a console application must not be given.
use std::path::{Path, PathBuf};

use symdev_core::{Project, RemotePath, Result};

use super::{CompileFlags, CompileIncludes, RustBuild, arg, io};
use crate::resources::SdkIncludeCaseFold;

impl RustBuild {
    /// Where the shim object for `source` goes: `build/shims/<stem>.o`, under the
    /// project's `build/` like everything else cargo and the linker produce.
    pub fn shim_object(&self, project: &Project, source: &Path) -> PathBuf {
        let stem = source.file_stem().unwrap_or_default();
        project
            .root
            .join("build/shims")
            .join(Path::new(stem).with_extension("o"))
    }

    /// The shim is compiled with **`GcceBuild`'s own C++ argv**, so it sees `gcce.h`,
    /// the GCC-12 varargs repair, `-D__PRODUCT_INCLUDE__` and every define a C++ project
    /// gets. That is the whole point: the shim is ordinary S60 C++, and the day the
    /// recorded compile line changes the shim moves with it.
    ///
    /// The source directory is the shim directory, so `#include "symrs_shim.h"` finds
    /// its neighbour, and nothing of the user's project is on the include path — the
    /// shim belongs to the SDK.
    /// A GUI application's shim adds two things the console one has no use for: the
    /// case-insensitive overlay of `epoc32/include`, without which `fbs.h`'s
    /// `#include <FbsMessage.h>` stops the Avkon header chain on a case-sensitive
    /// host, and `SYMRS_UID3`, the application's own UID3. The UID is generated onto
    /// the compile line rather than written into a source, because the manifest
    /// already holds it and two copies drift apart.
    pub fn shim_compile_args(
        &self,
        source: &Path,
        obj: &Path,
        casefold: Option<&Path>,
    ) -> Result<Vec<String>> {
        let includes = CompileIncludes {
            system: casefold.map(Path::to_path_buf).into_iter().collect(),
            ..CompileIncludes::default()
        };
        let flags = CompileFlags {
            macros: match self.ui {
                Some(_) => vec![format!("SYMRS_UID3=0x{:08x}", self.gcce.uid3)],
                None => Vec::new(),
            },
            option: Vec::new(),
        };
        let dir = match source.parent() {
            Some(dir) => dir.to_path_buf(),
            None => self.sdk.shim_dir(),
        };
        self.gcce.compile_args_for(
            &self.gcce.exe_module(),
            &flags,
            &dir,
            &includes,
            source,
            obj,
        )
    }

    /// `build/shims/libsymrs.a`: the shim as a static library.
    ///
    /// An **archive**, not a list of objects, and for one measured reason. Objects are
    /// linked whole, so an unused wrapper's reference to its DLL still makes ld record a
    /// `DT_NEEDED` — `--gc-sections` then removes the code but not the dependency,
    /// because as-needed is decided during symbol resolution and garbage collection
    /// happens after it. A `hello` that calls nothing came out at 3219 bytes and loaded
    /// `bafl.dll` for no reason. From an archive a member nobody references is never
    /// pulled and the question does not arise.
    pub fn shim_archive(&self, project: &Project) -> PathBuf {
        project.root.join("build/shims/libsymrs.a")
    }

    /// `ar cr <archive> <objects…>`, the one archiver invocation.
    pub fn ar_args(&self, archive: &Path, objects: &[PathBuf]) -> Result<Vec<String>> {
        let mut args = vec![arg(&self.gcce.tools.ar()?), "cr".into(), arg(archive)];
        args.extend(objects.iter().map(|o| arg(o)));
        Ok(args)
    }

    /// Compiles every SDK shim source into `build/shims/` and archives the objects.
    pub(super) fn build_shims(
        &self,
        project: &Project,
        cwd: &RemotePath,
    ) -> Result<Option<PathBuf>> {
        let sources = self.sdk.shim_sources(self.ui.is_some())?;
        if sources.is_empty() {
            return Ok(None);
        }
        let build_dir = project.root.join("build");
        // Only the Avkon headers need it, so a console application does not pay for
        // building the overlay at all.
        let casefold = match self.ui {
            Some(_) => Some(SdkIncludeCaseFold::ensure(
                &self.gcce.tools.epocroot.join("epoc32/include"),
                &build_dir.join("sdk-include-casefold"),
            )?),
            None => None,
        };
        std::fs::create_dir_all(project.root.join("build/shims")).map_err(io)?;
        let mut objects = Vec::new();
        for source in sources {
            let obj = self.shim_object(project, &source);
            self.gcce.run_tool(
                &self.shim_compile_args(&source, &obj, casefold.as_deref())?,
                cwd,
            )?;
            objects.push(obj);
        }
        let archive = self.shim_archive(project);
        // `ar cr` updates in place, so a stale member from an earlier build would
        // survive a renamed source; the archive is rebuilt from scratch every time.
        if archive.exists() {
            std::fs::remove_file(&archive).map_err(io)?;
        }
        self.gcce
            .run_tool(&self.ar_args(&archive, &objects)?, cwd)?;
        Ok(Some(archive))
    }
}
