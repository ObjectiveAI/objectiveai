//! One `codex exec` process: one turn, spoken to over its pipes.

use std::collections::BTreeMap;
use std::io;
use std::process::{ExitStatus, Stdio};

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

use crate::response::ThreadEvent;

/// The process, alive as long as this is: killed on drop, so an
/// abandoned run leaves no Codex behind.
pub struct Process {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
}

impl Process {
    /// Spawn one turn: `codex exec --json` with the sandbox and the
    /// approvals bypassed (the container is the sandbox), the git
    /// check skipped (the workspace is whatever the caller mounted),
    /// `resume <thread_id>` when the thread exists, and the prompt
    /// on stdin — `-` — written whole and closed, because argv caps a
    /// single argument and joined queue prompts can exceed it. The
    /// environment is the container's plus what the run rendered:
    /// `CODEX_HOME`, and the login's variable when the login is a
    /// key. stderr passes through.
    pub async fn start(
        env: &BTreeMap<String, String>,
        thread_id: Option<&str>,
        prompt: &str,
    ) -> io::Result<Self> {
        let mut command = Command::new("codex");
        command
            .arg("exec")
            .arg("--json")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("--skip-git-repo-check");
        if let Some(thread_id) = thread_id {
            command.arg("resume").arg(thread_id);
        }
        command
            .arg("-")
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let mut stdin = child.stdin.take().expect("stdin piped above");
        let stdout = child.stdout.take().expect("stdout piped above");
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.shutdown().await?;
        drop(stdin);
        Ok(Process {
            child,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// The next event; `None` once stdout is closed. A line that is
    /// not an event is the error, with the line in it.
    pub async fn next(&mut self) -> Result<Option<ThreadEvent>, LineError> {
        loop {
            match self.lines.next_line().await {
                Ok(None) => return Ok(None),
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    return serde_json::from_str(&line)
                        .map(Some)
                        .map_err(|error| LineError::Parse { line, error });
                }
                Err(error) => return Err(LineError::Io(error)),
            }
        }
    }

    /// Wait for the exit.
    pub async fn wait(mut self) -> io::Result<ExitStatus> {
        self.child.wait().await
    }
}

/// A line could not be read or understood.
#[derive(Debug)]
pub enum LineError {
    /// The pipe failed.
    Io(io::Error),
    /// The line is not an event.
    Parse {
        line: String,
        error: serde_json::Error,
    },
}

impl std::fmt::Display for LineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineError::Io(error) => write!(f, "codex's pipe failed: {error}"),
            LineError::Parse { line, error } => {
                write!(f, "codex wrote a line that is not an event ({error}): {line}")
            }
        }
    }
}

impl std::error::Error for LineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LineError::Io(error) => Some(error),
            LineError::Parse { error, .. } => Some(error),
        }
    }
}
