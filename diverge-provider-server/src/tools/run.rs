//! Running one program: waited for, waited for and read, or left
//! running.

use std::process::{ExitStatus, Stdio};

use tokio::process::{Child, Command};

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

/// Run `command`, wait for it, and keep what it wrote to stdout: for
/// the tools that answer. An exit other than success is
/// [`Error::Status`], since a tool that refused wrote no answer worth
/// reading; an answer that is not UTF-8 is [`Error::Output`]. Stdin
/// and the kill on drop are as [`run`]'s.
pub async fn capture(program: &str, mut command: Command) -> Result<String, Error> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|source| Error::Spawn {
            program: program.to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(Error::Status {
            program: program.to_string(),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    String::from_utf8(output.stdout).map_err(|_| Error::Output {
        program: program.to_string(),
    })
}

/// Start `command` and hand back the child, for a program that lives
/// longer than a call: the tunnel to the podman machine, the proxy
/// exec inside a container. Every stream is closed, so nothing it
/// writes is read and nothing it reads arrives; it is killed when the
/// child is dropped, so whoever holds the child holds the program's
/// life. Only the start can fail here; how it ends is the holder's
/// to watch.
pub fn start(program: &str, mut command: Command) -> Result<Child, Error> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| Error::Spawn {
            program: program.to_string(),
            source,
        })
}
