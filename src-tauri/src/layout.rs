use std::path::{Path, PathBuf};

use crate::error::Result;

/// Directory layout under the environment root:
///
/// ```text
/// <root>/bin/<runtime>/<version>/   downloaded runtimes
/// <root>/etc/<runtime>/             generated configs
/// <root>/www/                       project folders (one vhost each)
/// <root>/data/mariadb/              database files
/// <root>/logs/                      service logs
/// <root>/tmp/                       downloads, sessions, uploads
/// <root>/ssl/                       local CA and site certificates
/// ```
#[derive(Debug, Clone)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bin(&self, runtime: &str, version: &str) -> PathBuf {
        self.root.join("bin").join(runtime).join(version)
    }

    pub fn etc(&self, runtime: &str) -> PathBuf {
        self.root.join("etc").join(runtime)
    }

    pub fn www(&self) -> PathBuf {
        self.root.join("www")
    }

    /// Services the user added from a file: `services/<id>/service.yaml`.
    pub fn services(&self) -> PathBuf {
        self.root.join("services")
    }

    pub fn data(&self, runtime: &str) -> PathBuf {
        self.root.join("data").join(runtime)
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub fn ssl(&self) -> PathBuf {
        self.root.join("ssl")
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.root.join("bin"),
            self.root.join("etc"),
            self.www(),
            self.root.join("data"),
            self.logs(),
            self.tmp(),
            self.ssl(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}
