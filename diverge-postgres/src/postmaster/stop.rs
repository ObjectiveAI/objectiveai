//! Ending the postmaster a data directory names.

use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use tokio::process::Command;

use super::Error;
use crate::install::Binaries;

/// The file in the data directory that names the running postmaster.
const PID_FILE: &str = "postmaster.pid";

/// How long a fast stop is given before an immediate one is asked
/// for, and an immediate one before the signal.
const GRACE: Duration = Duration::from_secs(60);

/// How long after the signal the postmaster is given to be gone.
const AFTER_KILL: Duration = Duration::from_secs(10);

/// `pg_ctl status` exits with this when no postmaster runs on the data
/// directory.
const NOT_RUNNING: i32 = 3;

/// Stop whatever postmaster runs on `data`, and leave no pid file
/// behind.
///
/// Nothing to do when there is no pid file. With one, `pg_ctl status`
/// says whether the postmaster it names runs; a stale file — the
/// process is gone — is removed. A running one is asked to stop fast,
/// waiting; then immediately, waiting; then sent `KILL` and given a
/// moment; after each, `status` is asked again, and the first "not
/// running" is the answer. A postmaster still running after all
/// three is [`Error::StillRunning`].
pub async fn stop(binaries: &Binaries, data: &Path) -> Result<(), Error> {
    let pid_file = data.join(PID_FILE);
    if !tokio::fs::try_exists(&pid_file).await.unwrap_or(false) {
        return Ok(());
    }
    if !running(binaries, data).await? {
        return remove(&pid_file).await;
    }
    for mode in ["fast", "immediate"] {
        let seconds = GRACE.as_secs().to_string();
        let _ = pg_ctl(binaries, data, &["stop", "-m", mode, "-w", "-t", &seconds]).await?;
        if !running(binaries, data).await? {
            return remove(&pid_file).await;
        }
    }
    if let Some(pid) = pid(&pid_file).await {
        let _ = pg_ctl(binaries, data, &["kill", "KILL", &pid]).await?;
        let deadline = tokio::time::Instant::now() + AFTER_KILL;
        while tokio::time::Instant::now() < deadline {
            if !running(binaries, data).await? {
                return remove(&pid_file).await;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    Err(Error::StillRunning)
}

/// Whether a postmaster runs on `data`, as `pg_ctl status` says: exit
/// `0` is running, `3` is not, and anything else is a failure of
/// `pg_ctl` itself.
async fn running(binaries: &Binaries, data: &Path) -> Result<bool, Error> {
    let (status, stderr) = pg_ctl(binaries, data, &["status"]).await?;
    match status.code() {
        Some(0) => Ok(true),
        Some(NOT_RUNNING) => Ok(false),
        _ => Err(Error::PgCtl {
            action: "status".to_string(),
            status,
            stderr,
        }),
    }
}

/// Run `pg_ctl -D <data> <args>` and hand back how it exited and what
/// it said; only a `pg_ctl` that could not be started is an error
/// here, since whether an exit is a failure depends on what was
/// asked.
async fn pg_ctl(binaries: &Binaries, data: &Path, args: &[&str]) -> Result<(ExitStatus, String), Error> {
    let output = Command::new(binaries.pg_ctl())
        .arg("-D")
        .arg(data)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|source| Error::Spawn {
            program: "pg_ctl".to_string(),
            source,
        })?;
    Ok((output.status, String::from_utf8_lossy(&output.stderr).into_owned()))
}

/// The process id on the first line of the pid file, if the file
/// reads and the line is a number.
async fn pid(pid_file: &Path) -> Option<String> {
    let content = tokio::fs::read_to_string(pid_file).await.ok()?;
    let first = content.lines().next()?.trim();
    first.parse::<u32>().ok().map(|pid| pid.to_string())
}

/// Remove a pid file no postmaster is behind, so that the next start
/// is not refused by it.
async fn remove(pid_file: &Path) -> Result<(), Error> {
    match tokio::fs::remove_file(pid_file).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            path: pid_file.to_path_buf(),
            source,
        }),
    }
}
