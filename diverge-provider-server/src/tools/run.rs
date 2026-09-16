//! Running one program, and what it finished with.

use std::process::{ExitStatus, Stdio};

use tokio::process::Command;

use super::Error;

/// How a program finished: its status, and what it wrote to stderr.
#[derive(Debug)]
pub struct Finished {
    /// How it exited.
    pub status: ExitStatus,
    /// What it wrote to stderr, whole, for the error if there is one.
    pub stderr: String,
}

impl Finished {
    /// The exit judged: `Ok` where `accepted` says the status is one
    /// the caller takes, else [`Error::Status`] naming `program`.
    pub fn require(self, program: &str, accepted: impl Fn(&ExitStatus) -> bool) -> Result<(), Error> {
        if accepted(&self.status) {
            return Ok(());
        }
        Err(Error::Status {
            program: program.to_string(),
            status: self.status,
            stderr: self.stderr,
        })
    }
}

/// Run `command` and wait for it: the one place a child is spawned.
///
/// Stdin is closed, so a program that would prompt reads EOF; stdout
/// is discarded, since the tools narrate and nothing here reads what
/// they say; stderr is kept for the error. The child is killed if
/// this future is dropped, so a cancelled caller leaves nothing
/// running. `program` names it in the error, which is the tool the
/// caller asked for and not necessarily the first word of the
/// command — a tool run inside the podman machine is `podman`'s
/// child, and the error still names the tool.
pub async fn run(program: &str, mut command: Command) -> Result<Finished, Error> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|source| Error::Spawn {
            program: program.to_string(),
            source,
        })?;
    Ok(Finished {
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
