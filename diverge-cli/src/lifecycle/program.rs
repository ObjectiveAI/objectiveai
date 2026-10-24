//! Which program: the daemon, the provider, or the issuer.

use std::path::{Path, PathBuf};

/// One of the three programs the CLI starts and stops on its root. Each
/// is its own binary beside this one, holds its own lock file in the
/// root for as long as it runs, and is given its own log file there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    /// The daemon, `diverge-daemon`.
    Daemon,
    /// The provider, `diverge-provider`.
    Provider,
    /// The issuer, `diverge-issuer`.
    Issuer,
}

impl Program {
    /// The binary's file name: `diverge-daemon`, `diverge-provider` or
    /// `diverge-issuer`,
    /// with the host's executable suffix — `.exe` on Windows, nothing
    /// elsewhere.
    pub fn binary(self) -> String {
        let name = match self {
            Program::Daemon => "diverge-daemon",
            Program::Provider => "diverge-provider",
            Program::Issuer => "diverge-issuer",
        };
        format!("{name}{}", std::env::consts::EXE_SUFFIX)
    }

    /// The lock file the program holds in `root` for its life, named
    /// as the SDK names it for the program —
    /// [`daemon::LOCK_FILE`](diverge_sdk::daemon::LOCK_FILE) or
    /// [`provider::LOCK_FILE`](diverge_sdk::provider::LOCK_FILE) — or
    /// as the issuer's own crate names it,
    /// [`LOCK_FILE`](diverge_issuer::LOCK_FILE) — so that the CLI
    /// finds it where the program does.
    pub fn lock(self, root: &Path) -> PathBuf {
        match self {
            Program::Daemon => root.join(diverge_sdk::daemon::LOCK_FILE),
            Program::Provider => root.join(diverge_sdk::provider::LOCK_FILE),
            Program::Issuer => root.join(diverge_issuer::LOCK_FILE),
        }
    }

    /// The file in `root` a started program's stdout and stderr go
    /// to: `daemon.log`, `provider.log` or `issuer.log`, truncated at
    /// every start.
    pub fn log(self, root: &Path) -> PathBuf {
        match self {
            Program::Daemon => root.join("daemon.log"),
            Program::Provider => root.join("provider.log"),
            Program::Issuer => root.join("issuer.log"),
        }
    }

    /// The program's name in a line printed or an error: `daemon` or
    /// `provider` or `issuer`.
    pub fn word(self) -> &'static str {
        match self {
            Program::Daemon => "daemon",
            Program::Provider => "provider",
            Program::Issuer => "issuer",
        }
    }
}
