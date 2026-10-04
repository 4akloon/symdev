//! `Eka2l1`: the EKA2L1 symdev starts (emulator packages spec §5): the user's own, named by
//! `SYMDEV_EKA2L1` and started as it is, or an installed `emulator` package's program.
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eka2l1 {
    /// `SYMDEV_EKA2L1`: a binary or the user's wrapper, with the user's environment.
    User(PathBuf),
    /// `<package>/usr/bin/eka2l1_qt` of an installed `emulator` package (experiment 115 §1.2).
    Package(PathBuf),
}

impl Eka2l1 {
    /// What a packaged EKA2L1 must not inherit: these make the loader and Qt take the host's
    /// libraries and plugins before the bundled ones. Nothing else changes, so GL settings
    /// in the user's environment still reach it.
    pub const HOST_LIBRARY_VARIABLES: [&'static str; 3] = [
        "LD_LIBRARY_PATH",
        "QT_PLUGIN_PATH",
        "QT_QPA_PLATFORM_PLUGIN_PATH",
    ];

    pub fn program(&self) -> &Path {
        match self {
            Eka2l1::User(program) | Eka2l1::Package(program) => program,
        }
    }

    /// Readies `command`, which runs [`Self::program`] (directly or through `setsid`):
    /// the package's loses the host's library paths, the user's keeps everything.
    pub fn prepare(&self, command: &mut Command) {
        if let Eka2l1::Package(_) = self {
            for name in Self::HOST_LIBRARY_VARIABLES {
                command.env_remove(name);
            }
        }
    }

    /// Where it comes from, for messages.
    pub fn describe(&self) -> String {
        match self {
            Eka2l1::User(p) => format!("SYMDEV_EKA2L1 ({})", p.display()),
            Eka2l1::Package(p) => format!("the emulator package's {}", p.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::Eka2l1;

    fn removed(c: &Command) -> Vec<String> {
        let mut names: Vec<String> = c
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_packaged_eka2l1_does_not_inherit_the_hosts_library_paths() {
        let mut c = Command::new("setsid");
        Eka2l1::Package("/p/usr/bin/eka2l1_qt".into()).prepare(&mut c);
        assert_eq!(
            removed(&c),
            [
                "LD_LIBRARY_PATH",
                "QT_PLUGIN_PATH",
                "QT_QPA_PLATFORM_PLUGIN_PATH"
            ]
        );
    }

    #[test]
    fn the_users_eka2l1_keeps_its_environment() {
        let mut c = Command::new("setsid");
        Eka2l1::User("/home/u/.local/bin/eka2l1".into()).prepare(&mut c);
        assert_eq!(c.get_envs().count(), 0);
    }

    #[test]
    fn each_says_where_it_comes_from() {
        let user = Eka2l1::User("/u/eka2l1".into()).describe();
        assert_eq!(user, "SYMDEV_EKA2L1 (/u/eka2l1)");
        let package = Eka2l1::Package("/h/emulator/2026.10.04/usr/bin/eka2l1_qt".into()).describe();
        assert_eq!(
            package,
            "the emulator package's /h/emulator/2026.10.04/usr/bin/eka2l1_qt"
        );
    }
}
