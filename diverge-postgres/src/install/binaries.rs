//! The three programs, by where the extraction put them.

use std::path::{Path, PathBuf};

/// Where one extracted version of Postgres is, and the programs in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binaries {
    /// `bin/<version>/`, the extracted archive whole.
    home: PathBuf,
}

impl Binaries {
    /// The binaries under `home`, which is `bin/<version>/`.
    pub(super) fn new(home: PathBuf) -> Self {
        Binaries { home }
    }

    /// The server itself, started directly — never through `pg_ctl
    /// start`, which would daemonize it away from the supervisor.
    pub fn postgres(&self) -> PathBuf {
        self.program("postgres")
    }

    /// The one cross-platform way to ask a postmaster to stop, to ask
    /// whether one runs, and to signal it.
    pub fn pg_ctl(&self) -> PathBuf {
        self.program("pg_ctl")
    }

    /// What makes a cluster.
    pub fn initdb(&self) -> PathBuf {
        self.program("initdb")
    }

    /// `home/bin/<name>`, with `.exe` on Windows.
    fn program(&self, name: &str) -> PathBuf {
        let mut file = PathBuf::from(name);
        if cfg!(windows) {
            file.set_extension("exe");
        }
        Path::new(&self.home).join("bin").join(file)
    }
}
