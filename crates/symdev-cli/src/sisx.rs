//! `ProjectPackage`: a project's `.sisx` from an E32 image and the resources beside it.
use std::path::{Path, PathBuf};

use symdev_build::{AppTarget, SisPackage, UiResources};
use symdev_core::{Error, PackageBackend, Project, Result};
use symdev_manifest::Manifest;

use crate::artifacts::package_artifacts;

/// What `symdev package` and `symdev-ld` both do after the image exists: the manifest's
/// package, its resources from beside the image, and the signed `.sisx` written there.
pub(crate) struct ProjectPackage {
    manifest: Manifest,
    root: PathBuf,
    epocroot: PathBuf,
    app: String,
    uid3: u32,
}

impl ProjectPackage {
    pub fn new(manifest: Manifest, root: PathBuf, epocroot: PathBuf) -> Result<Self> {
        let uid3 = manifest
            .symbian
            .uid3
            .ok_or_else(|| Error::Other("uid3 required for package (set symbian.uid3)".into()))?;
        let project = Project { root: root.clone() };
        let app = AppTarget::of(&project, &manifest.package.name, &epocroot)?
            .name()
            .to_string();
        Ok(Self {
            manifest,
            root,
            epocroot,
            app,
            uid3,
        })
    }

    /// The package of a test binary: a console program even in an Avkon project (spec
    /// §4.5), so without the `[ui]` resources and icon the application's link builds.
    pub fn console(mut self) -> Self {
        self.manifest.ui = None;
        self.manifest.symbian.icon = None;
        self
    }

    /// The application's name: the image is `<app>.exe`.
    pub fn app(&self) -> &str {
        &self.app
    }

    /// Packages `exe` (named `<app>.exe`) with the resources in its directory, and writes
    /// `<name>.sis` and `<name>.sisx` there. Returns the `.sisx`.
    pub fn package(&self, exe: &Path, password: &str) -> Result<PathBuf> {
        let m = &self.manifest;
        let project = Project {
            root: self.root.clone(),
        };
        let icon = m.symbian.icon.clone();
        // The same caption translations the build compiled, so the package installs them.
        let locales = symdev_locale::Locales::load(&self.root.join("locales"))
            .map_err(|e| Error::Other(e.to_string()))?;
        let ui = m.ui.clone().map(|ui| {
            UiResources {
                app: self.app.clone(),
                uid3: self.uid3,
                ui,
                icon: icon.as_ref().map(|i| self.root.join(i)),
                captions: Vec::new(),
            }
            .with_locales(locales.as_ref())
        });
        let absolute = |p: &PathBuf| match p.is_absolute() {
            true => p.clone(),
            false => self.root.join(p),
        };
        let package = SisPackage {
            name: m.package.name.clone(),
            app: self.app.clone(),
            uid3: self.uid3,
            version: m.package.version,
            vendor: m.symbian.vendor.clone(),
            capabilities: m.symbian.capabilities.clone(),
            password: password.to_string(),
            cert: m.signing.cert.as_ref().map(absolute),
            key: m.signing.key.as_ref().map(absolute),
            subject: m.signing.subject.clone(),
        }
        .package(&package_artifacts(
            &project,
            exe,
            if ui.is_some() { None } else { icon.as_deref() },
            &m.icons,
            &m.install,
            &self.epocroot,
            ui.as_ref(),
        )?)?;
        Ok(package.primary)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use symdev_build::UiResources;
    use symdev_manifest::{Softkeys, UiApp, UiKind};

    use crate::artifacts::package_artifacts;

    #[test]
    fn resources_are_taken_from_beside_the_image() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("out/app.symdev");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("app.exe"), b"E32").unwrap();
        let project = symdev_core::Project {
            root: dir.path().to_path_buf(),
        };
        let got = package_artifacts(
            &project,
            &work.join("app.exe"),
            None,
            &[],
            &[],
            Path::new(""),
            None,
        )
        .unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, work.join("app.exe"));

        let ui = UiResources {
            app: "app".into(),
            uid3: 0xe123_4567,
            ui: UiApp {
                kind: UiKind::Avkon,
                caption: "App".into(),
                short_caption: "App".into(),
                softkeys: Softkeys::OptionsExit,
                left_softkey: "Options".into(),
                right_softkey: "Exit".into(),
            },
            icon: None,
            captions: Vec::new(),
        };
        for a in ui.artifacts(&work) {
            std::fs::write(&a.path, b"rsc").unwrap();
        }
        let got = package_artifacts(
            &project,
            &work.join("app.exe"),
            None,
            &[],
            &[],
            Path::new(""),
            Some(&ui),
        )
        .unwrap();
        assert!(got.len() > 1, "{got:?}");
        for a in &got {
            assert!(a.path.starts_with(&work), "{}", a.path.display());
        }
    }

    const AVKON: &str = "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[target]\n\
        device = \"nokia-e52\"\n\n[language]\nname = \"rust\"\n\n[symbian]\n\
        uid3 = \"0xe1234567\"\ncapabilities = []\nvendor = \"symdev\"\n\n[ui]\n\
        kind = \"avkon\"\ncaption = \"App\"\n\n[signing]\nmode = \"self-signed\"\n";

    /// A test binary is linked as a console program even in an Avkon project (spec §4.5),
    /// so it has none of the application's resources, and its package must not ask for them.
    #[test]
    fn a_test_of_an_avkon_project_is_packaged_as_a_console_program() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("out/smoke-0123456789abcdef.symdev");
        std::fs::create_dir_all(&work).unwrap();
        let exe = work.join("app.exe");
        std::fs::write(&exe, b"").unwrap();
        let m = symdev_manifest::parse(AVKON).unwrap();
        let root = dir.path().to_path_buf();
        let app = super::ProjectPackage::new(m.clone(), root.clone(), "".into()).unwrap();
        assert!(
            app.package(&exe, "").is_err(),
            "the app's own resources are not there"
        );
        let test = super::ProjectPackage::new(m, root, "".into())
            .unwrap()
            .console();
        assert!(test.package(&exe, "").unwrap().is_file());
    }
}
