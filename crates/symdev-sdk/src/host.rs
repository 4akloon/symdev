/// The machine an archive runs on. `Any` is for packages without host code (the SDK).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Host {
    #[serde(rename = "x86_64-linux")]
    X86_64Linux,
    #[serde(rename = "any")]
    Any,
}

impl Host {
    /// The host this symdev was built for, if packages exist for it.
    pub fn current() -> Option<Host> {
        cfg!(all(target_arch = "x86_64", target_os = "linux")).then_some(Host::X86_64Linux)
    }

    /// The name used in `index.toml`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Host::X86_64Linux => "x86_64-linux",
            Host::Any => "any",
        }
    }
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Host;

    #[test]
    fn names_match_the_index() {
        assert_eq!(Host::X86_64Linux.as_str(), "x86_64-linux");
        assert_eq!(Host::Any.as_str(), "any");
        assert_eq!(Host::Any.to_string(), "any");
    }

    #[test]
    fn this_build_host_is_x86_64_linux() {
        let expected =
            cfg!(all(target_arch = "x86_64", target_os = "linux")).then_some(Host::X86_64Linux);
        assert_eq!(Host::current(), expected);
        assert_ne!(Host::current(), Some(Host::Any));
    }
}
