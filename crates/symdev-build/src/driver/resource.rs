//! `GcceBuild`: compiling one `.rss` resource with the SDK `cpp.exe`/`rcomp.exe`.
use std::path::{Path, PathBuf};

use symdev_core::{Error, RemotePath, Result};

use super::{GcceBuild, arg};
use crate::{Mmp, MmpResource};

impl GcceBuild {
    pub(super) fn compile_resource(
        &self,
        res: &MmpResource,
        mmp_dir: &Path,
        mmp: &Mmp,
        build_dir: &Path,
        cwd: &RemotePath,
    ) -> Result<()> {
        use symdev_rcomp::{RcompTool, RssCppTool};
        let rss = res.source(mmp_dir);
        if !rss.is_file() {
            return Err(Error::Other(format!(
                "resource not found: {}",
                rss.display()
            )));
        }
        let stem = res.stem()?;
        let rpp = build_dir.join(format!("{stem}.rpp"));
        let rsc = build_dir.join(format!("{stem}.rsc"));
        let rsg = build_dir.join(format!("{stem}.rsg"));
        let epoc = self.tools.epocroot.join("epoc32");
        let mut includes = vec![
            rss.parent().unwrap_or(mmp_dir).to_path_buf(),
            build_dir.to_path_buf(),
        ];
        includes.extend(
            mmp.userinclude
                .iter()
                .map(|d| Self::mmp_dir_path(mmp_dir, d)),
        );
        includes.push(epoc.join("include"));
        includes.extend(
            mmp.systeminclude
                .iter()
                .map(|d| Self::mmp_dir_path(mmp_dir, d)),
        );
        let wine_env = [
            ("WINEPATH", arg(&epoc.join("tools"))),
            ("WINEDEBUG", "-all".into()),
        ];
        let cpp = RssCppTool::new(&self.tools.wine, &epoc.join("gcc/bin/cpp.exe"));
        self.run_tool_env(
            &cpp.args(
                &includes,
                &RssCppTool::wine_path(&rss),
                &RssCppTool::wine_path(&rpp),
            ),
            cwd,
            &wine_env,
        )?;
        let rcomp = RcompTool::new(&self.tools.wine, &epoc.join("tools/rcomp.exe"));
        let (o, s, i) = (
            RssCppTool::wine_path(&rsc),
            RssCppTool::wine_path(&rpp),
            RssCppTool::wine_path(&rss),
        );
        let args = if res.header {
            rcomp.args_with_header(&o, &RssCppTool::wine_path(&rsg), &s, &i)
        } else {
            rcomp.args(&o, &s, &i)
        };
        self.run_tool_env(&args, cwd, &wine_env)?;
        if !rsc.is_file() {
            return Err(Error::Other(format!("rcomp wrote no {}", rsc.display())));
        }
        Ok(())
    }

    /// MMP include directory (`..\\inc` style) relative to the MMP.
    pub(super) fn mmp_dir_path(mmp_dir: &Path, dir: &str) -> PathBuf {
        let dir = dir.replace('\\', "/");
        if Path::new(&dir).is_absolute() {
            PathBuf::from(dir)
        } else {
            mmp_dir.join(dir)
        }
    }
}
