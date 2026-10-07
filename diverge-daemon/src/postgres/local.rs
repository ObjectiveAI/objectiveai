//! The Postgres beside the daemon: `diverge-postgres`, started and
//! stopped.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use diverge_sdk::postgres_supervisor::{Command, Ready};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{Child, ChildStdin};

use super::Error;

/// The program's name beside the daemon's executable, with the
/// platform's suffix.
const PROGRAM: &str = "diverge-postgres";

/// How long the program is given to stop its cluster and end after
/// the shutdown line, before it is killed.
const STOP: Duration = Duration::from_secs(120);

/// The running `diverge-postgres`, held until the stop.
#[derive(Debug)]
pub struct Local {
    /// The program.
    child: Child,
    /// Its stdin, where the shutdown line goes.
    stdin: ChildStdin,
}

/// Start `diverge-postgres` on `<dir>/postgres/` and hand back it and
/// the URL of its cluster.
///
/// The program is the one beside the daemon's own executable. Its
/// stdout is read line by line until one is the
/// [`Ready`](diverge_sdk::postgres_supervisor::Ready) line, which
/// carries the URL; stdout ending first is the program having failed
/// to start, [`Error::Ended`], and what it said is on its stderr,
/// which is left to be the daemon's own. The program is not leashed:
/// nothing ends it when the daemon dies, and the daemon's next start
/// has the program stop what it left.
pub async fn spawn(dir: &Path) -> Result<(Local, String), Error> {
    let path = beside()?;
    let mut child = tokio::process::Command::new(&path)
        .arg("--config")
        .arg(dir.join("postgres"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(false)
        .spawn()
        .map_err(|source| Error::Spawn { path, source })?;
    let stdin = child.stdin.take().expect("stdin was asked for piped");
    let stdout = child.stdout.take().expect("stdout was asked for piped");
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next_line().await.map_err(Error::Stdout)? {
            Some(line) => {
                if let Ok(Ready { url }) = serde_json::from_str::<Ready>(&line) {
                    return Ok((Local { child, stdin }, url));
                }
            }
            None => {
                let _ = child.wait().await;
                return Err(Error::Ended);
            }
        }
    }
}

impl Local {
    /// Stop the cluster: the shutdown line on the program's stdin,
    /// then the wait for the program to end, and a kill when it has
    /// not after a generous while. Nothing of this can be reported to
    /// anyone, so nothing is.
    pub async fn stop(mut self) {
        let mut line = serde_json::to_vec(&Command::Shutdown).unwrap_or_default();
        line.push(b'\n');
        let _ = self.stdin.write_all(&line).await;
        let _ = self.stdin.flush().await;
        drop(self.stdin);
        if tokio::time::timeout(STOP, self.child.wait()).await.is_err() {
            let _ = self.child.kill().await;
        }
    }
}

/// `diverge-postgres` beside the daemon's own executable.
fn beside() -> Result<PathBuf, Error> {
    let own = std::env::current_exe().map_err(Error::Executable)?;
    let dir = own.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok(dir.join(format!("{PROGRAM}{}", std::env::consts::EXE_SUFFIX)))
}
