//! A `file://` source built inside a test: packed archives and the `index.toml` that
//! lists them.

use std::fs;
use std::path::PathBuf;

use crate::{ArchiveEntry, Auth, Host, Index, IndexPackage, PackageId, ReproducibleTarGz};
use crate::{SdkHome, SourceSpec};

pub struct Repo {
    dir: PathBuf,
    index: Index,
}

impl Repo {
    pub fn new(dir: PathBuf) -> Repo {
        fs::create_dir_all(&dir).unwrap();
        Repo {
            dir,
            index: Index::empty(),
        }
    }

    /// Adds `id` for `host`, holding one file `marker` that names the id and this repo.
    pub fn add(&mut self, id: &str, host: Host, depends: &[&str]) -> &mut Repo {
        let id = PackageId::parse(id).unwrap();
        let tree = self.dir.join(".tree").join(id.relative_path());
        fs::create_dir_all(&tree).unwrap();
        fs::write(
            tree.join("marker"),
            format!("{id} from {}", self.dir.display()),
        )
        .unwrap();
        let packed = self.dir.join("packed.tar.gz");
        let (sha256, size) = ReproducibleTarGz::pack(&tree, &["."], &packed).unwrap();
        let url = format!("{}/{sha256}.tar.gz", id.relative_path().display());
        fs::create_dir_all(self.dir.join(&url).parent().unwrap()).unwrap();
        fs::rename(&packed, self.dir.join(&url)).unwrap();
        let package = IndexPackage {
            id,
            license: "MIT".into(),
            source_code: None,
            depends: depends
                .iter()
                .map(|d| PackageId::parse(d).unwrap())
                .collect(),
            archives: vec![ArchiveEntry {
                host,
                url,
                sha256,
                size,
            }],
        };
        self.index.insert(package).unwrap();
        self.write_index();
        self
    }

    pub fn write_index(&self) {
        fs::write(self.index_path(), self.index.to_toml().unwrap()).unwrap();
    }

    pub fn index_path(&self) -> PathBuf {
        self.dir.join("index.toml")
    }

    pub fn source(&self, name: &str) -> SourceSpec {
        SourceSpec::new(name, &format!("file://{}", self.dir.display()), Auth::None).unwrap()
    }

    /// The text of `marker` in the installed `id`.
    pub fn marker(home: &SdkHome, id: &str) -> String {
        let id = PackageId::parse(id).unwrap();
        fs::read_to_string(home.package_dir(&id).join("marker")).unwrap()
    }
}
