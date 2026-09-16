//! A podman invocation.

use std::ffi::OsStr;

use tokio::process::Command;

use super::super::{Finished, run};
use crate::tools::Error;

/// `podman` with `args`, as a command not yet run: the root every
/// podman invocation grows from, so what podman is called and how is
/// decided here once. Podman is named bare and found on `PATH`.
pub fn command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("podman");
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
