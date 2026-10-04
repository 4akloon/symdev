//! `RustProject`: a Rust project's link, provisioned the way `symdev build` provisions it,
//! for `symdev build` and `symdev-ld` alike.
use std::path::{Path, PathBuf};

use symdev_build::{GcceBuild, RustBuild, UiResources};
use symdev_core::{Error, LocalEnv, Result};
use symdev_manifest::Manifest;

use crate::provision::Provision;

pub(crate) struct RustProject {
    pub build: RustBuild,
    pub epocroot: PathBuf,
}

impl RustProject {
    /// `ui = false` links a console program even when the manifest has a `[ui]` section:
    /// a test binary has no Avkon application around it (design spec §4.5).
    pub fn resolve(m: &Manifest, root: &Path, provision: &Provision, ui: bool) -> Result<Self> {
        let uid3 = m
            .symbian
            .uid3
            .ok_or_else(|| Error::Other("uid3 required for build (set symbian.uid3)".into()))?;
        // Resolved before the toolchain, so a Rust project without its Rust SDK is told so
        // before the compiler is downloaded.
        let sdk = provision.rust_sdk()?;
        let linker = provision.rust_linker()?;
        // GCCE is left out only when rust-lld links with the Rust SDK's prebuilt set.
        let gcce = linker.needs_gcce(&sdk)?;
        if let Some(note) = provision.prebuilt_note(&sdk, &linker)? {
            eprintln!("{note}");
        }
        let tools = provision.toolchain(m.target.device, gcce)?;
        let epocroot = tools.epocroot.clone();
        // A `[ui]` project's icon is built by the Rust backend's own resource stage,
        // which names it after the application rather than after an MMP target there is
        // none of; `GcceBuild` must not also try, or `AppIcon::of` fails looking for one.
        let icon = m.symbian.icon.clone();
        // `locales/` may translate the caption; the launcher reads each translation from
        // its own `<app>.r<code>`, so the resource stage needs to know them.
        let locales = symdev_locale::Locales::load(&root.join("locales"))
            .map_err(|e| Error::Other(e.to_string()))?;
        let ui = m.ui.clone().filter(|_| ui).map(|ui| {
            UiResources {
                app: m.package.name.clone(),
                uid3,
                ui,
                icon: icon.as_ref().map(|i| root.join(i)),
                captions: Vec::new(),
            }
            .with_locales(locales.as_ref())
        });
        let gcce = GcceBuild {
            env: LocalEnv,
            tools,
            uid3,
            capabilities: m.symbian.capabilities.clone(),
            icon: if m.ui.is_some() { None } else { icon },
            icons: m.icons.clone(),
            secure_id: m.symbian.secure_id,
        };
        let build = RustBuild {
            gcce,
            sdk,
            cargo: RustBuild::cargo_from_env(),
            rustc: RustBuild::rustc_from_env(),
            name: m.package.name.clone(),
            linker,
            ui,
            std: m.language.has_std(),
        };
        Ok(Self { build, epocroot })
    }
}
