//! A temporary world for `symdev sdk` and auto-install tests: its own `SYMDEV_HOME`,
//! cache and config, and a `file://` source named `local` whose packages the test packs.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use symdev_sdk::{ArchiveEntry, Host, Index, IndexPackage, PackageId, ReproducibleTarGz};

pub struct World {
    pub tmp: tempfile::TempDir,
    index: Index,
}

impl World {
    /// An empty `local` source, listed in this world's `sources.toml`.
    pub fn new() -> World {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("repo")).unwrap();
        let world = World {
            tmp,
            index: Index::empty(),
        };
        world.write_index();
        let local = format!(
            "builtin = false\n\n[[source]]\nname = \"local\"\nurl = \"file://{}\"\n",
            world.repo().display()
        );
        world.sources(&local);
        world
    }

    /// Replaces this world's `sources.toml`.
    pub fn sources(&self, text: &str) {
        let config = self.tmp.path().join("config/symdev");
        fs::create_dir_all(&config).unwrap();
        fs::write(config.join("sources.toml"), text).unwrap();
    }

    /// Packs the files `files` (relative path, contents, executable) as `id` for `host`.
    pub fn add(&mut self, id: &str, host: Host, files: &[(&str, &str, bool)]) {
        let id = PackageId::parse(id).unwrap();
        let tree = self.tmp.path().join("trees").join(id.relative_path());
        for (path, text, executable) in files {
            let path = tree.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            if *executable {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        let packed = self.repo().join("packed.tar.gz");
        let (sha256, size) = ReproducibleTarGz::pack(&tree, &["."], &packed).unwrap();
        let url = format!("{}/{sha256}.tar.gz", id.relative_path().display());
        fs::create_dir_all(self.repo().join(&url).parent().unwrap()).unwrap();
        fs::rename(&packed, self.repo().join(&url)).unwrap();
        let archives = vec![ArchiveEntry {
            host,
            url,
            sha256,
            size,
        }];
        let package = IndexPackage {
            id,
            license: "MIT".into(),
            source_code: None,
            depends: vec![],
            archives,
        };
        self.index.insert(package).unwrap();
        self.write_index();
    }

    /// A GCCE whose compiler and linker are scripts that print `stub <tool>` and fail.
    pub fn add_stub_gcce(&mut self) {
        let stub = |tool: &str| format!("#!/bin/sh\necho \"stub {tool} $*\" >&2\nexit 1\n");
        let (gxx, ld) = (stub("g++"), stub("ld"));
        self.add(
            "gcce;12.1.0",
            Host::X86_64Linux,
            &[
                ("bin/arm-none-symbianelf-g++", &gxx, true),
                ("bin/arm-none-symbianelf-ld", &ld, true),
                ("lib/gcc/arm-none-symbianelf/12.1.0/libgcc.a", "", false),
                ("arm-none-symbianelf/lib/libsupc++.a", "", false),
            ],
        );
    }

    /// An SDK with the variant files a `bld.inf` needs and one header.
    pub fn add_stub_sdk(&mut self) {
        self.add(
            "sdk;s60-3rd-fp2;1.1",
            Host::Any,
            &[
                ("epoc32/include/e32def.h", "", false),
                (
                    "epoc32/include/variant/Symbian_OS_v9.3.hrh",
                    "#define __SERIES60_3X__\n",
                    false,
                ),
                (
                    "epoc32/tools/variant/variant.cfg",
                    "epoc32\\include\\variant\\Symbian_OS_v9.3.hrh\n",
                    false,
                ),
            ],
        );
    }

    /// `symdev` in this world, with no toolchain variable set.
    pub fn bin(&self) -> Command {
        let mut cmd = super::bin_without_toolchain();
        cmd.env("SYMDEV_HOME", self.home())
            .env("XDG_DATA_HOME", self.tmp.path().join("data"))
            .env("XDG_CACHE_HOME", self.tmp.path().join("cache"))
            .env("XDG_CONFIG_HOME", self.tmp.path().join("config"));
        cmd
    }

    pub fn home(&self) -> PathBuf {
        self.tmp.path().join("home")
    }

    pub fn repo(&self) -> PathBuf {
        self.tmp.path().join("repo")
    }

    pub fn package_dir(&self, id: &str) -> PathBuf {
        self.home()
            .join(PackageId::parse(id).unwrap().relative_path())
    }

    fn write_index(&self) {
        fs::write(
            self.repo().join("index.toml"),
            self.index.to_toml().unwrap(),
        )
        .unwrap();
    }

    /// A C++ project with one source file and a `bld.inf`, in its own directory.
    pub fn project(&self) -> PathBuf {
        let dir = self.tmp.path().join("project");
        let file = |path: &str, text: &str| {
            let path: &Path = &dir.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        file("symdev.toml", &super::hello_with_uid3());
        file("group/bld.inf", "PRJ_MMPFILES\nhello.mmp\n");
        file(
            "group/hello.mmp",
            "TARGET hello.exe\nTARGETTYPE exe\nUID 0 0xE0000001\nSOURCEPATH ../src\n\
             SOURCE hello.cpp\nLIBRARY euser.lib\n",
        );
        file("src/hello.cpp", "int E32Main() { return 0; }\n");
        dir
    }
}
