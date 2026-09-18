use std::path::PathBuf;

use symdev_core::{Error, Result};

#[derive(Debug)]
pub struct Signsis {
    pub sis: PathBuf,
    pub sisx: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
    pub password: String,
}

impl Signsis {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let tokens = match args.first().map(String::as_str) {
            Some(s) if !s.starts_with('-') && !s.ends_with(".sis") && !s.ends_with(".sisx") => {
                &args[1..]
            }
            _ => args,
        };
        if let Some(flag) = tokens.first().filter(|t| t.starts_with('-')) {
            // TODO: signsis -? -h -c* -i -o -p -s -u -v (recorded usage; not Wave 0)
            return Err(Error::Other(format!("TODO: signsis {flag}")));
        }
        match tokens {
            [sis, sisx, cert, key, password] => Ok(Self {
                sis: PathBuf::from(sis),
                sisx: PathBuf::from(sisx),
                cert: PathBuf::from(cert),
                key: PathBuf::from(key),
                password: password.clone(),
            }),
            _ => Err(Error::Other(
                "Usage: signsis input output certificate key passphrase".into(),
            )),
        }
    }

    pub fn run(&self) -> Result<()> {
        let _ = self;
        // TODO: inflate unsigned SIS and attach signatures (Wave 0 package uses SisUnsigned::encode_signed)
        Err(Error::Other(
            "TODO: inflate unsigned SIS and attach signatures".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| (*t).to_string()).collect()
    }

    #[test]
    fn from_args_match_experiment_8() {
        let s = Signsis::from_args(&args(&[
            "signsis",
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret",
        ]))
        .unwrap();
        assert_eq!(s.sis, PathBuf::from("hello.sis"));
        assert_eq!(s.sisx, PathBuf::from("hello.sisx"));
        assert_eq!(s.cert, PathBuf::from("hello.cer"));
        assert_eq!(s.key, PathBuf::from("hello.key"));
        assert_eq!(s.password, "secret");
    }

    #[test]
    fn from_args_todo_unused_flags() {
        let err = Signsis::from_args(&args(&["signsis", "-v", "hello.sis"])).unwrap_err();
        assert!(err.to_string().contains("TODO: signsis -v"));
    }
}
