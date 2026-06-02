use std::path::PathBuf;
use std::pin::Pin;

use futures::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::cli::command::{CommandExecutor, CommandRequest};

/// Spawn the `objectiveai` cli binary on disk, feed it the argv from
/// `request.into_command()`, and stream each stdout JSONL line back as
/// either a typed `T` or a structured [`crate::cli::Error`].
///
/// The binary is resolved at `execute` time (not at construction) so the
/// home-dir lookup only happens when actually needed:
/// `<config_base_dir>/objectiveai{.exe}` if `config_base_dir` is set,
/// otherwise `<home>/.objectiveai/objectiveai{.exe}`.
pub struct BinaryExecutor {
    config_base_dir: Option<PathBuf>,
}

impl BinaryExecutor {
    pub fn new(config_base_dir: Option<impl Into<PathBuf>>) -> Self {
        Self {
            config_base_dir: config_base_dir.map(Into::into),
        }
    }

    fn binary_path(&self) -> Result<PathBuf, Error> {
        let base = match &self.config_base_dir {
            Some(d) => d.clone(),
            None => dirs::home_dir()
                .ok_or(Error::NoHomeDir)?
                .join(".objectiveai"),
        };
        let name = if cfg!(windows) { "objectiveai.exe" } else { "objectiveai" };
        Ok(base.join(name))
    }
}

#[derive(Debug)]
pub enum Error {
    /// `dirs::home_dir()` returned `None` and no `config_base_dir` was set.
    NoHomeDir,
    /// Failed to spawn the binary at the resolved path.
    Spawn(std::io::Error),
    /// Child stdout was unexpectedly absent after spawn.
    NoStdout,
    /// Reading stdout line failed.
    Io(std::io::Error),
    /// Stdout produced a line that didn't deserialize as either the
    /// structured `cli::Error` or `T`.
    Json(serde_json::Error),
    /// Structured error emitted by the cli binary on stdout.
    Cli(crate::cli::Error),
}

/// Per-line untagged decode. `Err` is listed first so serde tries it
/// before `Ok` — `cli::Error`'s `type:"error"` constant short-circuits
/// every non-error wire shape, then `Ok(T)` is the fallthrough.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Line<T> {
    Err(crate::cli::Error),
    Ok(T),
}

impl<T> From<Line<T>> for Result<T, Error> {
    fn from(line: Line<T>) -> Self {
        match line {
            Line::Err(e) => Err(Error::Cli(e)),
            Line::Ok(t) => Ok(t),
        }
    }
}

impl CommandExecutor for BinaryExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(&self, request: R) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send,
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let argv = request.into_command();
        let binary = self.binary_path()?;

        let mut child = Command::new(&binary)
            .args(&argv)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(Error::Spawn)?;

        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        let lines = BufReader::new(stdout).lines();

        // Carry `child` in the unfold state so its handle isn't dropped
        // mid-stream. tokio's Child defaults to kill_on_drop = false,
        // so on stream drop the child remains running until it finishes.
        let stream = futures::stream::unfold(
            (child, lines),
            |(child, mut lines)| async move {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let item = match serde_json::from_str::<Line<T>>(&line) {
                            Ok(line) => line.into(),
                            Err(e) => Err(Error::Json(e)),
                        };
                        Some((item, (child, lines)))
                    }
                    Ok(None) => None,
                    Err(e) => Some((Err(Error::Io(e)), (child, lines))),
                }
            },
        );

        Ok(Box::pin(stream))
    }
}
