use std::path::PathBuf;
use std::pin::Pin;

use futures::{Stream, StreamExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::cli::command::{AgentArguments, CommandExecutor, CommandRequest, CommandResponse};

/// Spawn the `objectiveai` cli binary on disk, feed it the argv from
/// `request.into_command()`, and stream each stdout JSONL line back as
/// either a typed `T` or a structured [`crate::cli::Error`].
///
/// The binary is resolved at `execute` time (not at construction).
/// Resolution order:
///
/// - [`BinaryExecutor::from_path`] — returns the explicit path verbatim
///   (used by tests pointing at an out-of-tree build).
/// - [`BinaryExecutor::new`] with `Some(config_base_dir)` —
///   `<config_base_dir>/objectiveai{.exe}`.
/// - [`BinaryExecutor::new`] with `None` —
///   `<home>/.objectiveai/objectiveai{.exe}`.
pub struct BinaryExecutor {
    /// Resolves to a directory; the binary inside is always
    /// `objectiveai{.exe}`. `None` falls back to the home-dir default.
    config_base_dir: Option<PathBuf>,
    /// When set, used verbatim and the `config_base_dir` lookup is
    /// skipped entirely. Lets callers (notably tests) point the
    /// executor at any binary by absolute path without enforcing the
    /// `<dir>/objectiveai` naming convention.
    explicit_path: Option<PathBuf>,
    /// Extra environment variables to set on the spawned child. Stacks
    /// on top of the parent's environment (the child inherits the rest
    /// — this map only overrides). Set via [`Self::env`].
    extra_env: Vec<(String, String)>,
    /// When `true`, the spawned child is sent a kill signal whenever
    /// the [`tokio::process::Child`] held in the stream state is
    /// dropped. Defaults to `false` (the SDK's long-standing
    /// behaviour: drop-without-kill lets the child finish its work
    /// orphaned). Set via [`Self::kill_on_drop`]. Test wrappers that
    /// want to abort hung children (e.g. cli-test
    /// `HangPreventingBinaryCommandExecutor`) flip this on.
    kill_on_drop: bool,
}

impl BinaryExecutor {
    pub fn new(config_base_dir: Option<impl Into<PathBuf>>) -> Self {
        Self {
            config_base_dir: config_base_dir.map(Into::into),
            explicit_path: None,
            extra_env: Vec::new(),
            kill_on_drop: false,
        }
    }

    /// Construct an executor that spawns the binary at `binary`
    /// directly, regardless of file name. Skips the `objectiveai`
    /// name lookup so tests can target a `target/debug/objectiveai-cli`
    /// build without renaming or symlinking.
    pub fn from_path(binary: impl Into<PathBuf>) -> Self {
        Self {
            config_base_dir: None,
            explicit_path: Some(binary.into()),
            extra_env: Vec::new(),
            kill_on_drop: false,
        }
    }

    /// Set an environment variable on every child the executor spawns.
    /// Stacks on top of the parent's env; intended for tests that need
    /// to pin a per-instance `CONFIG_BASE_DIR` or `OBJECTIVEAI_ADDRESS`
    /// without racing other parallel tests via `std::env::set_var`.
    pub fn env(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.extra_env.push((key.into(), value.into()));
        self
    }

    /// When `true`, the spawned [`tokio::process::Child`] held in the
    /// stream state is sent a kill signal when the stream is dropped.
    /// Defaults to `false`. Wrappers that detect hangs (e.g. the
    /// cli-test `HangPreventingBinaryCommandExecutor`) toggle this on
    /// so dropping the inner stream tears the cli child down.
    pub fn kill_on_drop(mut self, on: bool) -> Self {
        self.kill_on_drop = on;
        self
    }

    fn binary_path(&self) -> Result<PathBuf, Error> {
        if let Some(p) = &self.explicit_path {
            return Ok(p.clone());
        }
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `dirs::home_dir()` returned `None` and no `config_base_dir` was set.
    #[error("no home directory and no config_base_dir set")]
    NoHomeDir,
    /// Failed to spawn the binary at the resolved path.
    #[error("failed to spawn cli binary: {0}")]
    Spawn(std::io::Error),
    /// Child stdout was unexpectedly absent after spawn.
    #[error("cli binary child has no stdout handle")]
    NoStdout,
    /// Reading stdout line failed.
    #[error("read cli binary stdout: {0}")]
    Io(std::io::Error),
    /// Stdout produced a line that didn't deserialize as either the
    /// structured `cli::Error` or `T`.
    #[error("decode cli binary stdout line: {0}")]
    Json(serde_json::Error),
    /// Structured error emitted by the cli binary on stdout.
    #[error("{0}")]
    Cli(crate::cli::Error),
    /// `execute_one` was called but the stream produced no items.
    #[error("cli binary stream produced no items")]
    Empty,
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

    async fn execute<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let argv = request.into_command();
        let binary = self.binary_path()?;

        let mut command = Command::new(&binary);
        command
            .args(&argv)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(self.kill_on_drop);
        for (k, v) in &self.extra_env {
            command.env(k, v);
        }
        // Per-call agent identity override. When `Some`, every field
        // gets applied atomically: `Some(v)` → set, `None` →
        // env_remove so the parent's value can't leak through. When
        // the bag itself is `None`, parent env is inherited
        // untouched.
        if let Some(args) = agent_arguments {
            args.apply_to_command(&mut command);
        }
        let mut child = command.spawn().map_err(Error::Spawn)?;

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

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, agent_arguments).await?;
        stream.next().await.ok_or(Error::Empty)?
    }
}
