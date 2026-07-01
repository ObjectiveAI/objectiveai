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
/// - [`BinaryExecutor::new`] with `Some(objectiveai_dir)` —
///   `<objectiveai_dir>/bin/objectiveai{.exe}`.
/// - [`BinaryExecutor::new`] with `None` —
///   `<home>/.objectiveai/bin/objectiveai{.exe}`.
pub struct BinaryExecutor {
    /// The layout root (`OBJECTIVEAI_DIR`); the binary inside is
    /// always `bin/objectiveai{.exe}`. `None` falls back to the
    /// home-dir default (`~/.objectiveai`).
    objectiveai_dir: Option<PathBuf>,
    /// When set, used verbatim and the `objectiveai_dir` lookup is
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
    /// When `true`, spawn the child detached from the parent's console
    /// / process group so it survives the parent process exiting.
    /// Unix already does this automatically when the parent dies
    /// (kernel re-parents to init). Windows needs explicit
    /// `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` creation flags;
    /// otherwise the child's inherited console closes with the parent
    /// and the child gets a `CTRL_CLOSE_EVENT`. Defaults to `false`.
    detach: bool,
    /// One-shot lockfile claim to hand off to the next spawned child
    /// ([`crate::lockfile::LockClaim::prepare_transfer`] before the
    /// spawn, [`crate::lockfile::LockClaim::transfer`] after) — the
    /// child becomes the sole owner and the lock lives until the
    /// child exits. Consumed by the first `execute`; interior
    /// mutability because [`CommandExecutor::execute`] takes `&self`.
    /// Set via [`Self::transfer_lock`].
    #[cfg(feature = "lockfile")]
    transfer_locks: std::sync::Mutex<Vec<crate::lockfile::LockClaim>>,
}

impl BinaryExecutor {
    pub fn new(objectiveai_dir: Option<impl Into<PathBuf>>) -> Self {
        Self {
            objectiveai_dir: objectiveai_dir.map(Into::into),
            explicit_path: None,
            extra_env: Vec::new(),
            kill_on_drop: false,
            detach: false,
            #[cfg(feature = "lockfile")]
            transfer_locks: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Construct an executor that spawns the binary at `binary`
    /// directly, regardless of file name. Skips the `objectiveai`
    /// name lookup so tests can target a `target/debug/objectiveai-cli`
    /// build without renaming or symlinking.
    pub fn from_path(binary: impl Into<PathBuf>) -> Self {
        Self {
            objectiveai_dir: None,
            explicit_path: Some(binary.into()),
            extra_env: Vec::new(),
            kill_on_drop: false,
            detach: false,
            #[cfg(feature = "lockfile")]
            transfer_locks: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Hand `claim` off to the next spawned child: ownership of the
    /// lock transfers into the child process, which keeps it until it
    /// exits (the parent retains nothing). Accumulates with any prior
    /// claim(s); all are consumed by the first `execute`. On spawn
    /// failure the claims are released; on transfer failure the child
    /// is killed best-effort, the failing claim plus every not-yet-
    /// transferred claim are released, and `execute` returns
    /// [`Error::LockTransfer`] — either way the lock slots are retryable.
    #[cfg(feature = "lockfile")]
    pub fn transfer_lock(mut self, claim: crate::lockfile::LockClaim) -> Self {
        self.transfer_locks
            .get_mut()
            .expect("transfer_locks mutex poisoned")
            .push(claim);
        self
    }

    /// Hand a whole set of claims off to the next spawned child — see
    /// [`Self::transfer_lock`]. The entire family transfers into the child,
    /// which keeps them until it exits.
    #[cfg(feature = "lockfile")]
    pub fn transfer_locks(mut self, claims: Vec<crate::lockfile::LockClaim>) -> Self {
        self.transfer_locks
            .get_mut()
            .expect("transfer_locks mutex poisoned")
            .extend(claims);
        self
    }

    /// Set an environment variable on every child the executor spawns.
    /// Stacks on top of the parent's env; intended for tests that need
    /// to pin a per-instance `OBJECTIVEAI_DIR`/`OBJECTIVEAI_STATE` or `OBJECTIVEAI_ADDRESS`
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

    /// When `true`, the child is spawned detached from the parent's
    /// console / process group so it survives parent exit. Pairs
    /// naturally with `kill_on_drop = false` (the default): drop the
    /// stream + exit the parent, the child keeps running orphaned.
    ///
    /// - **Windows**: sets `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`
    ///   on the `CreateProcessW` call. `DETACHED_PROCESS` is what
    ///   actually makes orphan survival work — without it the child
    ///   inherits the parent's console and receives `CTRL_CLOSE_EVENT`
    ///   when the parent's console window closes. The process-group
    ///   flag is belt-and-suspenders signal isolation.
    /// - **Unix**: no-op. The kernel automatically re-parents the
    ///   child to init when the parent exits.
    ///
    /// Defaults to `false`.
    pub fn detach(mut self, on: bool) -> Self {
        self.detach = on;
        self
    }

    fn binary_path(&self) -> Result<PathBuf, Error> {
        if let Some(p) = &self.explicit_path {
            return Ok(p.clone());
        }
        let dir = match &self.objectiveai_dir {
            Some(d) => d.clone(),
            None => dirs::home_dir()
                .ok_or(Error::NoHomeDir)?
                .join(".objectiveai"),
        };
        let name = if cfg!(windows) { "objectiveai.exe" } else { "objectiveai" };
        Ok(dir.join("bin").join(name))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `dirs::home_dir()` returned `None` and no `objectiveai_dir` was set.
    #[error("no home directory and no objectiveai_dir set")]
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
    /// Transferring the lockfile claim into the spawned child failed.
    /// The child was killed best-effort and the claim released.
    #[cfg(feature = "lockfile")]
    #[error("transfer lockfile claim into cli binary child: {0}")]
    LockTransfer(std::io::Error),
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
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        // Dispatch the typed request as JSON via the cli's top-level
        // `--request` flag — request -> argv lowering no longer exists.
        let argv = vec![
            "--request".to_string(),
            serde_json::to_string(&request).map_err(Error::Json)?,
        ];
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
        // Windows detach. Required for orphan-survival when the
        // parent will exit before the child finishes — without
        // `DETACHED_PROCESS` the child inherits the parent's
        // console and dies on `CTRL_CLOSE_EVENT` when the
        // parent's console closes. We've hit this bug twice in
        // the past; preserve the flag. Unix gets re-parent-to-init
        // for free.
        #[cfg(windows)]
        if self.detach {
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
        }
        // Per-call agent identity override. When `Some`, every field
        // gets applied atomically: `Some(v)` → set, `None` →
        // env_remove so the parent's value can't leak through. When
        // the bag itself is `None`, parent env is inherited
        // untouched.
        if let Some(args) = agent_arguments {
            args.apply_to_command(&mut command);
        }
        // Lockfile-claim handoff, step 1 of 2: arm the command so the
        // child inherits/duplicates each claim's handles at spawn.
        // `prepare_transfer` accumulates (env + unix CLOEXEC hooks stack),
        // so multiple claims hand off to the one child.
        #[cfg(feature = "lockfile")]
        let transfer_claims: Vec<crate::lockfile::LockClaim> = std::mem::take(
            &mut *self
                .transfer_locks
                .lock()
                .expect("transfer_locks mutex poisoned"),
        );
        #[cfg(feature = "lockfile")]
        for claim in &transfer_claims {
            // Arms the command: CLOEXEC-clear (unix) + the inherited-lock
            // env so the child adopts + re-acquires the claim instantly.
            claim.prepare_transfer(&mut command);
        }
        let spawned = command.spawn();
        // Step 2 of 2: complete (or unwind) the handoff. Dropping a
        // claim does NOT release it (ManuallyDrop), so every failure
        // path must release explicitly to keep the lock slots
        // retryable. On a mid-set transfer failure the already-handed
        // claims belong to the child; release the failing claim + every
        // remaining (un-transferred) one and kill the child.
        #[cfg(feature = "lockfile")]
        let spawned = match spawned {
            Ok(child) => {
                let mut remaining = transfer_claims.into_iter();
                let mut transfer_err = None;
                for claim in remaining.by_ref() {
                    if let Err((claim, e)) = claim.transfer(&child) {
                        let _ = claim.release();
                        transfer_err = Some(e);
                        break;
                    }
                }
                if let Some(e) = transfer_err {
                    for claim in remaining {
                        let _ = claim.release();
                    }
                    let mut child = child;
                    let _ = child.start_kill();
                    return Err(Error::LockTransfer(e));
                }
                Ok(child)
            }
            Err(e) => {
                for claim in transfer_claims {
                    let _ = claim.release();
                }
                Err(e)
            }
        };
        let mut child = spawned.map_err(Error::Spawn)?;

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
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, agent_arguments).await?;
        stream.next().await.ok_or(Error::Empty)?
    }
}
