//! [`Podman`] — a self-contained podman runtime rooted at a bin
//! directory. This is the state `objectiveai-cli`'s `Context` used to
//! hold inline (the lazy install memo + the in-process machine
//! serialization), extracted so ANY consumer — the CLI, the Tauri
//! viewer — can own a runtime with nothing but a writable directory.

use std::path::{Path, PathBuf};

use super::Error;

/// A podman runtime rooted at `bin_dir`.
///
/// - The **install** (download + extract into `<bin_dir>/podman/<version>/`,
///   [`super::install`]) runs once and is memoized here; concurrent
///   in-process callers coalesce on the `OnceCell`, cross-process
///   callers on the bin lock.
/// - The global machine (macOS VM / Windows WSL2; no-op on Linux) is
///   ensured *running* on EVERY [`Self::executable`] call
///   ([`super::running`] — `machine init` if absent, `machine start`
///   if stopped); the `machine` mutex serializes that slow path
///   within this process.
///
/// Lazy on purpose: constructing a `Podman` costs nothing; callers
/// that never need podman never pay for it.
pub struct Podman {
    bin_dir: PathBuf,
    exe: tokio::sync::OnceCell<PathBuf>,
    machine: tokio::sync::Mutex<()>,
}

impl Podman {
    pub fn new(bin_dir: PathBuf) -> Self {
        Self {
            bin_dir,
            exe: tokio::sync::OnceCell::new(),
            machine: tokio::sync::Mutex::new(()),
        }
    }

    /// The podman executable, ready to use: installed if missing, its
    /// machine running.
    pub async fn executable(&self) -> Result<&Path, Error> {
        let exe = self
            .exe
            .get_or_try_init(|| super::install::ensure_installed(self.bin_dir.clone()))
            .await?;
        super::running::ensure_running(&self.machine, &self.bin_dir, exe).await?;
        Ok(exe.as_path())
    }

    /// The bin directory this runtime is rooted at.
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }
}
