use std::path::{Path, PathBuf};

use symdev_core::Error;

use crate::cli::Template;

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

pub fn create_project(cwd: &Path, name: &str, template: Template) -> Result<PathBuf, Error> {
    let root = cwd.join(name);
    if root.exists() {
        return Err(Error::Other(format!("directory `{name}` already exists")));
    }
    std::fs::create_dir_all(root.join("group")).map_err(io_err)?;
    std::fs::create_dir_all(root.join("src")).map_err(io_err)?;
    let uid3 = uid3_hex(name);
    let icon = match template {
        Template::Gui => format!("icon = \"gfx/{name}.svg\"\n"),
        Template::Console => String::new(),
    };
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
             {icon}\
             \n\
             [signing]\n\
             mode = \"self-signed\"\n"
        ),
    )
    .map_err(io_err)?;
    if template == Template::Gui {
        write_gui(&root, name, &uid3)?;
        return Ok(root);
    }
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

/// Avkon GUI app: sources, application + registration resources, MMP with LIBRARY.
fn write_gui(root: &Path, name: &str, uid3: &str) -> Result<(), Error> {
    let fill = |t: &str| t.replace("{{NAME}}", name).replace("{{UID3}}", uid3);
    std::fs::create_dir_all(root.join("data")).map_err(io_err)?;
    std::fs::create_dir_all(root.join("gfx")).map_err(io_err)?;
    let files = [
        (
            "group/bld.inf".to_string(),
            format!("PRJ_MMPFILES\n{name}.mmp\n"),
        ),
        (
            format!("group/{name}.mmp"),
            fill(include_str!("../templates/gui/app.mmp")),
        ),
        (
            format!("src/{name}.cpp"),
            fill(include_str!("../templates/gui/app.cpp")),
        ),
        (
            format!("data/{name}.rss"),
            fill(include_str!("../templates/gui/app.rss")),
        ),
        (
            format!("data/{name}_reg.rss"),
            fill(include_str!("../templates/gui/app_reg.rss")),
        ),
        (
            format!("gfx/{name}.svg"),
            fill(include_str!("../templates/gui/app.svg")),
        ),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).map_err(io_err)?;
    }
    Ok(())
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
        let root = create_project(&dir, "hello", Template::Console).unwrap();
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
        let root = create_project(&dir, "hello", Template::Console).unwrap();
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
    fn examples_gui_matches_scaffold() {
        let dir = scratch();
        let root = create_project(&dir, "gui", Template::Gui).unwrap();
        let example: [(&str, &str); 7] = [
            (
                "symdev.toml",
                include_str!("../../../examples/gui/symdev.toml"),
            ),
            (
                "gfx/gui.svg",
                include_str!("../../../examples/gui/gfx/gui.svg"),
            ),
            (
                "group/bld.inf",
                include_str!("../../../examples/gui/group/bld.inf"),
            ),
            (
                "group/gui.mmp",
                include_str!("../../../examples/gui/group/gui.mmp"),
            ),
            (
                "src/gui.cpp",
                include_str!("../../../examples/gui/src/gui.cpp"),
            ),
            (
                "data/gui.rss",
                include_str!("../../../examples/gui/data/gui.rss"),
            ),
            (
                "data/gui_reg.rss",
                include_str!("../../../examples/gui/data/gui_reg.rss"),
            ),
        ];
        for (path, want) in example {
            let got = std::fs::read_to_string(root.join(path)).unwrap();
            assert_eq!(
                got, want,
                "examples/gui/{path} drifted from `symdev new --template gui`"
            );
        }
    }

    #[test]
    fn create_gui_project_fills_name_and_uid3() {
        let dir = scratch();
        let root = create_project(&dir, "notes", Template::Gui).unwrap();
        let uid = uid3_hex("notes");
        let mmp = std::fs::read_to_string(root.join("group/notes.mmp")).unwrap();
        assert!(mmp.contains("TARGET notes.exe"));
        assert!(mmp.contains("START RESOURCE notes_reg.rss"));
        assert!(mmp.contains(&format!("UID 0x100039CE {uid}")));
        let reg = std::fs::read_to_string(root.join("data/notes_reg.rss")).unwrap();
        assert!(reg.contains("#include <notes.rsg>"));
        assert!(reg.contains(&format!("UID3 {uid}")));
        let cpp = std::fs::read_to_string(root.join("src/notes.cpp")).unwrap();
        assert!(cpp.contains(&format!("static_cast<TInt32>({uid})")));
        for text in [&mmp, &reg, &cpp] {
            assert!(!text.contains("{{"), "unfilled template: {text}");
        }
        let toml = std::fs::read_to_string(root.join("symdev.toml")).unwrap();
        assert!(toml.contains("name = \"notes\""));
    }

    #[test]
    fn create_project_existing_dir_errors() {
        let dir = scratch();
        std::fs::create_dir(dir.join("hello")).unwrap();
        let err = create_project(&dir, "hello", Template::Console).unwrap_err();
        assert_eq!(err.to_string(), "directory `hello` already exists");
    }
}
