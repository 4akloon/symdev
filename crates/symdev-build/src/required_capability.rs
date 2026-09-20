//! `RequiredCapability`: the capabilities an image's imports make necessary, checked
//! against `[symbian] capabilities` before the E32 is written.
//!
//! Platform security is one of the things §6a says must not be hidden: there is no
//! `std` analogue, and an application declares its capabilities in `symdev.toml`. What
//! *can* be hidden is the diagnostic. Without this check, a Rust program that calls
//! `symbian_std::net` and forgets the manifest line builds, packages, installs and then
//! fails at `RSocket::Open` with a bare `-46` on a phone with no console — which is
//! exactly the "runtime error nobody can read" this repository tries not to ship.
//!
//! The check is on the **linked ELF's `DT_NEEDED` list**, because that is the observable
//! fact: the SDK puts `esock.dso` and `insock.dso` on every Rust link line under
//! `--as-needed`, so the dependency appears only when the program really reached for a
//! socket (experiment 78 measured that mechanism, and `hello` still has six `NEEDED`).
//!
//! It runs for **Rust** builds only. A `.mmp` project already states its capabilities in
//! the `.mmp` and [`crate::mmp::MmpCapabilities`] cross-checks that against the manifest;
//! a Rust project has no `.mmp`, and the SDK rather than the project decides what is
//! linked, so this is the only place the two can be brought together.
use std::path::Path;

use symdev_core::{Error, Result};

/// One rule: a DLL whose presence in the imports means a capability is required, and
/// where the SDK says so.
struct Rule {
    /// The `DT_NEEDED` name before its `{version}` suffix — `esock{000a0000}.dso` is
    /// `esock`.
    dll: &'static str,
    capability: &'static str,
    /// The SDK's own words, quoted in the error so a reader can check it.
    because: &'static str,
}

/// The capability rules this SDK knows, each taken from a header rather than recalled.
pub struct RequiredCapability;

impl RequiredCapability {
    const RULES: &'static [Rule] = &[
        Rule {
            dll: "esock",
            capability: "NetworkServices",
            because: "in_sock.h lines 66 and 74: \
                      `@capability NetworkServices Required for opening 'tcp' sockets. \
                      @ref RSocket::Open()`, and the same for 'udp'",
        },
        Rule {
            dll: "insock",
            capability: "NetworkServices",
            because: "in_sock.h lines 66 and 74: \
                      `@capability NetworkServices Required for opening 'tcp' sockets. \
                      @ref RSocket::Open()`, and the same for 'udp'",
        },
    ];

    /// Reads `elf`'s `DT_NEEDED` list and fails if it needs a capability `granted` does
    /// not contain.
    pub fn check(elf: &Path, granted: &[String], target: &str) -> Result<()> {
        let bytes =
            std::fs::read(elf).map_err(|e| Error::Other(format!("read {}: {e}", elf.display())))?;
        let needed = symdev_elf2e32::ElfImage::parse(bytes)?.needed_dlls()?;
        Self::check_needed(&needed, granted, target)
    }

    /// The decision itself, over a `DT_NEEDED` list, so it can be tested without a
    /// linker.
    pub fn check_needed(needed: &[String], granted: &[String], target: &str) -> Result<()> {
        for rule in Self::RULES {
            let Some(dll) = needed.iter().find(|n| Self::stem(n) == rule.dll) else {
                continue;
            };
            if granted.iter().any(|g| g == rule.capability) {
                continue;
            }
            return Err(Error::Other(format!(
                "{target} imports {dll}, which needs the `{}` capability, but \
                 `[symbian] capabilities` in symdev.toml does not grant it. Add it:\n\
                 \n    [symbian]\n    capabilities = [\"{}\"]\n\n\
                 Why: {}. It is a user-grantable capability, so a self-signed SIS may \
                 carry it and the phone asks the user at install time.",
                rule.capability, rule.capability, rule.because
            )));
        }
        Ok(())
    }

    /// `esock{000a0000}.dso` → `esock`; `euser.dso` → `euser`.
    fn stem(needed: &str) -> &str {
        needed.split(['{', '.']).next().unwrap_or(needed)
    }
}

#[cfg(test)]
mod tests {
    use super::RequiredCapability as R;

    fn needed() -> Vec<String> {
        ["euser{000a0000}.dso", "esock{000a0000}.dso"]
            .map(str::to_string)
            .to_vec()
    }

    #[test]
    fn an_import_of_esock_without_the_capability_names_it_and_the_manifest_key() {
        let e = R::check_needed(&needed(), &[], "netdemo.exe").unwrap_err();
        let text = e.to_string();
        assert!(text.contains("esock{000a0000}.dso"), "{text}");
        assert!(text.contains("NetworkServices"), "{text}");
        assert!(
            text.contains("capabilities = [\"NetworkServices\"]"),
            "{text}"
        );
        assert!(text.contains("in_sock.h"), "{text}");
    }

    #[test]
    fn the_granted_capability_passes() {
        R::check_needed(&needed(), &["NetworkServices".into()], "netdemo.exe").unwrap();
    }

    #[test]
    fn an_image_that_imports_no_socket_dll_needs_nothing() {
        let plain = ["euser{000a0000}.dso".to_string(), "efsrv.dso".to_string()];
        R::check_needed(&plain, &[], "hello.exe").unwrap();
    }

    #[test]
    fn insock_alone_is_enough_to_ask() {
        let only = ["insock{000a0000}.dso".to_string()];
        assert!(R::check_needed(&only, &[], "x.exe").is_err());
    }
}
