use std::path::{Path, PathBuf};

use symdev_core::Error;

pub fn uid3_for_name(name: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in name.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x01000193);
    }
    0xE0000000 | (h & 0x0FFFFFFF)
}

pub fn uid3_hex(name: &str) -> String {
    format!("0x{:08x}", uid3_for_name(name))
}

pub fn create_project(cwd: &Path, name: &str) -> Result<PathBuf, Error> {
    let root = cwd.join(name);
    if root.exists() {
        return Err(Error::Other(format!("directory `{name}` already exists")));
    }
    std::fs::create_dir_all(root.join("group")).map_err(io_err)?;
    std::fs::create_dir_all(root.join("src")).map_err(io_err)?;
    let uid3 = uid3_hex(name);
    std::fs::write(
        root.join("symdev.toml"),
        format!(
            "[package]\n\
             name = \"{name}\"\n\
             version = \"0.1.0\"\n\
             \n\
             [target]\n\
             device = \"nokia-e52\"\n\
             \n\
             [language]\n\
             name = \"cpp\"\n\
             \n\
             [symbian]\n\
             uid3 = \"{uid3}\"\n\
             capabilities = []\n\
             vendor = \"symdev\"\n\
             \n\
             [signing]\n\
             mode = \"self-signed\"\n"
        ),
    )
    .map_err(io_err)?;
    std::fs::write(
        root.join("group/bld.inf"),
        format!("PRJ_MMPFILES\n{name}.mmp\n"),
    )
    .map_err(io_err)?;
    std::fs::write(
        root.join(format!("group/{name}.mmp")),
        format!("TARGET {name}.exe\nTARGETTYPE EXE\nSOURCEPATH ../src\nSOURCE hello.cpp\n"),
    )
    .map_err(io_err)?;
    std::fs::write(
        root.join("src/hello.cpp"),
        include_str!("../templates/hello.cpp"),
    )
    .map_err(io_err)?;
    std::fs::write(
        root.join("src/hello.h"),
        include_str!("../templates/hello.h"),
    )
    .map_err(io_err)?;
    Ok(root)
}

fn io_err(e: std::io::Error) -> Error {
    Error::Other(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn uid3_hello_is_pinned() {
        assert_eq!(uid3_for_name("hello"), 0xef9f2cab);
        assert_eq!(uid3_hex("hello"), "0xef9f2cab");
    }

    fn scratch() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "symdev-scaffold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn create_project_writes_hello_tree() {
        let dir = scratch();
        let root = create_project(&dir, "hello").unwrap();
        assert_eq!(root, dir.join("hello"));
        let toml = std::fs::read_to_string(root.join("symdev.toml")).unwrap();
        assert!(toml.contains("name = \"hello\""));
        assert!(toml.contains("uid3 = \"0xef9f2cab\""));
        assert_eq!(
            std::fs::read_to_string(root.join("group/bld.inf")).unwrap(),
            "PRJ_MMPFILES\nhello.mmp\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("group/hello.mmp")).unwrap(),
            "TARGET hello.exe\nTARGETTYPE EXE\nSOURCEPATH ../src\nSOURCE hello.cpp\n"
        );
        assert_eq!(
            std::fs::read(root.join("src/hello.cpp")).unwrap(),
            include_bytes!("../templates/hello.cpp")
        );
        assert_eq!(
            std::fs::read(root.join("src/hello.h")).unwrap(),
            include_bytes!("../templates/hello.h")
        );
    }

    #[test]
    fn examples_hello_matches_scaffold() {
        let dir = scratch();
        let root = create_project(&dir, "hello").unwrap();
        let example: [(&str, &str); 5] = [
            (
                "symdev.toml",
                include_str!("../../../examples/hello/symdev.toml"),
            ),
            (
                "group/bld.inf",
                include_str!("../../../examples/hello/group/bld.inf"),
            ),
            (
                "group/hello.mmp",
                include_str!("../../../examples/hello/group/hello.mmp"),
            ),
            (
                "src/hello.cpp",
                include_str!("../../../examples/hello/src/hello.cpp"),
            ),
            (
                "src/hello.h",
                include_str!("../../../examples/hello/src/hello.h"),
            ),
        ];
        for (path, want) in example {
            let got = std::fs::read_to_string(root.join(path)).unwrap();
            assert_eq!(got, want, "examples/hello/{path} drifted from `symdev new`");
        }
    }

    #[test]
    fn create_project_existing_dir_errors() {
        let dir = scratch();
        std::fs::create_dir(dir.join("hello")).unwrap();
        let err = create_project(&dir, "hello").unwrap_err();
        assert_eq!(err.to_string(), "directory `hello` already exists");
    }
}
