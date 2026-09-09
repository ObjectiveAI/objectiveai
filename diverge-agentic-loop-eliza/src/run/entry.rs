//! The entry process: `node entry.mjs`, spoken to over its pipes.

use std::collections::BTreeMap;
use std::io;
use std::process::{ExitStatus, Stdio};

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::protocol::{Request, Response};
use crate::plugins::PROJECT;

/// The entry script, in the image's Node project.
pub const ENTRY: &str = "/opt/diverge/eliza/entry.mjs";

/// The entry, alive as long as this is: killed on drop, so an
/// abandoned run leaves no runtime behind.
pub struct Entry {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
}

impl Entry {
    /// Spawn `node entry.mjs` in the project, with exactly the
    /// environment rendered from the agent on top of the container's
    /// own; stdin and stdout are this program's, stderr is Eliza's
    /// logger and passes through.
    pub fn start(env: &BTreeMap<String, String>) -> io::Result<Self> {
        let mut child = Command::new("node")
            .arg(ENTRY)
            .current_dir(PROJECT)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child.stdin.take().expect("stdin piped above");
        let stdout = child.stdout.take().expect("stdout piped above");
        Ok(Entry {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// Write one request line.
    pub async fn send(&mut self, request: &Request) -> io::Result<()> {
        let mut line = serde_json::to_vec(request).map_err(io::Error::other)?;
        line.push(b'\n');
        self.stdin.write_all(&line).await?;
        self.stdin.flush().await
    }

    /// The next response line; `None` once stdout is closed. A line
    /// that is not a response is the error, with the line in it.
    pub async fn next(&mut self) -> Result<Option<Response>, LineError> {
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

    /// Close stdin and wait for the exit.
    pub async fn wait(self) -> io::Result<ExitStatus> {
        let Entry {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        child.wait().await
    }

    /// Kill the entry and reap it: for an entry that stopped
    /// answering, which nothing graceful reaches any more.
    pub async fn kill(self) -> io::Result<ExitStatus> {
        let Entry {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        child.kill().await?;
        child.wait().await
    }
}

/// A line could not be read or understood.
#[derive(Debug)]
pub enum LineError {
    /// The pipe failed.
    Io(io::Error),
    /// The line is not a response.
    Parse {
        line: String,
        error: serde_json::Error,
    },
}

impl std::fmt::Display for LineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineError::Io(error) => write!(f, "the entry's pipe failed: {error}"),
            LineError::Parse { line, error } => {
                write!(f, "the entry wrote a line that is not a response ({error}): {line}")
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
