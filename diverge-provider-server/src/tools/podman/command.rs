//! A podman invocation.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::OnceLock;

use tokio::process::Command;

use super::super::{Finished, run};
use crate::tools::Error;

/// Where podman keeps its data, once the configuration has said:
/// what every invocation is told. Set once, before the first
/// invocation, by [`configure`].
static STORAGE: OnceLock<PathBuf> = OnceLock::new();

/// Tell every podman invocation to come where podman's data is:
/// `containers.podman.storage_path`, resolved. Once; a second call
/// changes nothing.
pub fn configure(storage: PathBuf) {
    let _ = STORAGE.set(storage);
}

/// `podman` with `args`, as a command not yet run: the root every
/// podman invocation grows from, so what podman is called and how is
/// decided here once. Podman is named bare and found on `PATH`. The
/// storage [`configure`] gave is on every invocation: on Linux as
/// `--root`, before the subcommand, so the store is that directory;
/// on macOS and Windows as `XDG_DATA_HOME`, under which podman keeps
/// a machine's disk, so the machine the provider makes lives there.
pub fn command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("podman");
    if let Some(storage) = STORAGE.get() {
        #[cfg(target_os = "linux")]
        command.arg("--root").arg(storage);
        #[cfg(not(target_os = "linux"))]
        command.env("XDG_DATA_HOME", storage);
    }
    command.args(args);
    command
}

/// `podman` with `args`, run and waited for.
pub async fn podman<I, S>(args: I) -> Result<Finished, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run("podman", command(args)).await
}
