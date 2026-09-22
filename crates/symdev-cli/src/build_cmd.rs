//! `symdev build`: one backend per manifest language.
use std::process::ExitCode;

use symdev_build::{FrozenExports, GcceBuild, RustBuild, RustSdk, Toolchain, UiResources};
use symdev_core::{BuildBackend, Error, LocalEnv};
use symdev_manifest::{Language, Manifest};

pub fn build_project(m: Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for build (set symbian.uid3)".into()))?;
    if m.ui.is_some() && !m.language.is_rust() {
        return Err(Error::Other(
            "[ui] is for a `language = \"rust\"` project: a C++ project declares its \
             application resources in its .mmp with START RESOURCE, and symdev would \
             generate a second, conflicting pair from this section"
                .into(),
        ));
    }
    let tools = Toolchain::from_env()?;
    let epocroot = tools.epocroot.clone();
    let project = crate::current_project()?;
    ignore_build_dir(&project.root)?;
    // A `[ui]` project's icon is built by the Rust backend's own resource stage,
    // which names it after the application rather than after an MMP target there is
    // none of; `GcceBuild` must not also try, or `AppIcon::of` fails looking for one.
    let icon = m.symbian.icon.clone();
    // `locales/` may translate the caption; the launcher reads each translation from
    // its own `<app>.r<code>`, so the resource stage needs to know them.
    let locales = symdev_locale::Locales::load(&project.root.join("locales"))
        .map_err(|e| Error::Other(e.to_string()))?;
    let ui = m.ui.map(|ui| {
        UiResources {
            app: m.package.name.clone(),
            uid3,
            ui,
            icon: icon.as_ref().map(|i| project.root.join(i)),
            captions: Vec::new(),
        }
        .with_locales(locales.as_ref())
    });
    let gcce = GcceBuild {
        env: LocalEnv,
        tools,
        uid3,
        capabilities: m.symbian.capabilities,
        icon: if ui.is_some() { None } else { icon },
        icons: m.icons,
        secure_id: m.symbian.secure_id,
    };
    let artifacts = match m.language {
        Language::Cpp => gcce.build(&project)?,
        language => RustBuild {
            gcce,
            sdk: RustSdk::from_env()?,
            cargo: RustBuild::cargo_from_env(),
            rustc: RustBuild::rustc_from_env(),
            name: m.package.name,
            ui,
            std: language.has_std(),
        }
        .build(&project)?,
    };
    for artifact in artifacts {
        println!("{}", artifact.path.display());
    }
    if m.language != Language::Cpp {
        return Ok(ExitCode::SUCCESS);
    }
    for dll in FrozenExports::of(&project, &epocroot)? {
        if !dll.unfrozen.is_empty() {
            eprintln!(
                "warning: {}: {} export(s) not frozen in {} ({}); run `symdev freeze` \
                 before shipping so their ordinals stay fixed",
                dll.dll,
                dll.unfrozen.len(),
                dll.frozen_def.display(),
                dll.unfrozen.join(", ")
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Makes `build/` invisible to git before anything is written into it.
///
/// `build/` holds `sdk-include-casefold/`, a tree of symlinks into `SYMDEV_EPOCROOT`'s
/// headers, beside the objects and images. A project's own `.gitignore` is the
/// project's business, and one that does not name `build/` — any project outside this
/// repository's `examples/` — would commit those links with the first `git add -A`.
/// That happened: a branch here once added 1 044 of them. So the directory ignores
/// itself, the way a build tool's output directory should. An existing
/// `build/.gitignore` is left alone: if someone wrote one, it is theirs.
fn ignore_build_dir(root: &std::path::Path) -> Result<(), Error> {
    let dir = root.join("build");
    let io = |e: std::io::Error| Error::Other(format!("{}: {e}", dir.display()));
    std::fs::create_dir_all(&dir).map_err(io)?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(
            &ignore,
            "# Written by symdev: everything here is build output.\n*\n",
        )
        .map_err(io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ignore_build_dir;

    #[test]
    fn the_build_directory_ignores_itself() {
        let root = tempfile::tempdir().unwrap();
        ignore_build_dir(root.path()).unwrap();
        let text = std::fs::read_to_string(root.path().join("build/.gitignore")).unwrap();
        assert!(text.lines().any(|l| l == "*"));
    }

    #[test]
    fn an_existing_ignore_file_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("build")).unwrap();
        std::fs::write(root.path().join("build/.gitignore"), "mine\n").unwrap();
        ignore_build_dir(root.path()).unwrap();
        let text = std::fs::read_to_string(root.path().join("build/.gitignore")).unwrap();
        assert_eq!(text, "mine\n");
    }
}
