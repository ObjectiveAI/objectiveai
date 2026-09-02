//! What a ClaudeCode continuation token holds: a filesystem.
//!
//! Claude Code keeps its conversation on disk — append-only JSONL
//! under `$CLAUDE_CONFIG_DIR/projects/<sanitized-cwd>/<session>.jsonl`
//! plus its adjacent files, with no locks and no indexes (see
//! `cc-source-findings/HISTORY.md`). Resuming is those files existing
//! and `claude --resume <session id>` being launched with the same
//! working directory. So this container's continuation IS the
//! files: the continuation carries the session id and everything
//! needed to make it resumable, and running a continuation means
//! writing them back before Claude Code starts. On the wire it is
//! raw bytes — this JSON, uncoated: no base64, no envelope.
//!
//! The container fixes its geometry — one working directory, one
//! config dir (the stock `~/.claude`, no environment overrides) — so
//! every relative path in a continuation minted by one run resolves
//! identically in the next. The harness's flow: fresh runs launch
//! Claude Code and harvest `projects/**` into a continuation;
//! resumed runs [`write`](Continuation::write) its files and launch
//! with `--resume` and its session id.

use std::io;
use std::path::{Component, Path};

/// Where the session state lives, fixed for the container's life:
/// Claude Code's own default, `~/.claude` for the container's root
/// user. The environment stays stock — no `CLAUDE_CONFIG_DIR` — so
/// what [`Continuation::write`] lays down is what Claude Code finds.
pub const CONFIG_DIR: &str = "/root/.claude";

/// A continuation, opened.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Continuation {
    /// The session to hand to `claude --resume`.
    pub session_id: String,
    /// The files that make the session resumable, relative to the
    /// Claude config directory.
    pub files: Vec<ContinuationFile>,
}

/// One file of the session's state.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContinuationFile {
    /// Relative path under the config directory, `/`-separated —
    /// `projects/<dir>/<session>.jsonl`,
    /// `projects/<dir>/<session>/subagents/agent-<id>.jsonl`, and so
    /// on.
    pub path: String,
    /// The file's content, verbatim: the transcript family is UTF-8
    /// JSONL text. Binary adjacents (the image cache) are not
    /// carried.
    pub content: String,
}

impl Continuation {
    /// Open the chunks the server delivered: joined, they are the
    /// state as JSON. The protocol keeps the chunks apart for
    /// containers that put meaning in the boundaries; this one does
    /// not — its closer is one document split at the chunk ceiling,
    /// and joining is the whole of reading it back.
    pub fn parse(chunks: &[Vec<u8>]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(&chunks.concat())
    }

    /// The state as the bytes the run closes with —
    /// [`parse`](Self::parse)'s exact inverse.
    pub fn tokenize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Write every file under [`CONFIG_DIR`], parents created as
    /// needed. Safe to do wholesale before Claude Code starts:
    /// nothing else is reading, and the format has no index or lock
    /// to maintain.
    ///
    /// A path that is absolute, names a root, or contains a `..`
    /// component is refused with [`io::ErrorKind::InvalidInput`] —
    /// the continuation names files inside the config directory,
    /// and does not get to escape it.
    pub async fn write(&self) -> io::Result<()> {
        let config_dir = Path::new(CONFIG_DIR);
        for file in &self.files {
            let relative = Path::new(&file.path);
            if !relative.components().all(|component| {
                matches!(component, Component::Normal(_) | Component::CurDir)
            }) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "a continuation file escapes the config \
                         directory: {}",
                        file.path
                    ),
                ));
            }

            let path = config_dir.join(relative);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, &file.content).await?;
        }
        Ok(())
    }

    /// Harvest one session's on-disk state into a [`Continuation`].
    ///
    /// The inverse of [`write`](Continuation::write), called when a run
    /// ends: the session id arrives from Claude Code's own output (its
    /// stream-json chunks carry it), and everything the session left
    /// under [`CONFIG_DIR`] is swept up:
    ///
    /// - `projects/<dir>/<session>.jsonl` — found by NAME in every
    ///   project directory, rather than recomputing Claude Code's cwd
    ///   sanitizer;
    /// - `projects/<dir>/<session>/**` — subagents, tool results,
    ///   session memory, remote agents;
    /// - `file-history/<session>/**` — the edit backups the transcript's
    ///   checkpoint lines reference;
    /// - `tasks/<session>/**` — todo state, whose list id falls back to
    ///   the session id.
    ///
    /// Not harvested, deliberately: `history.jsonl` (global prompt
    /// history, cross-session), `image-cache` and `uploads` (binary,
    /// which the token does not carry), `debug` (diagnostics), plan
    /// documents (their slug lives inside transcript lines), and the
    /// config file. A file that is not valid UTF-8 is skipped for the
    /// same binary reason.
    ///
    /// Absence is not an error: missing directories contribute nothing,
    /// and a session that wrote no files yields an empty `files` — the
    /// caller judges that. `Err` is a real IO failure.
    pub async fn read(session_id: String) -> io::Result<Self> {
        let config_dir = Path::new(CONFIG_DIR);
        let mut files = Vec::new();

        // The transcripts: each project directory may hold this
        // session — worktrees make several possible.
        let projects = config_dir.join("projects");
        if let Ok(mut dirs) = tokio::fs::read_dir(&projects).await {
            while let Some(dir) = dirs.next_entry().await? {
                if !dir.file_type().await?.is_dir() {
                    continue;
                }
                let transcript =
                    dir.path().join(format!("{session_id}.jsonl"));
                collect_file(config_dir, &transcript, &mut files).await?;
                collect_tree(
                    config_dir,
                    dir.path().join(&session_id),
                    &mut files,
                )
                .await?;
            }
        }

        // The session-keyed families beside the transcripts.
        collect_tree(
            config_dir,
            config_dir.join("file-history").join(&session_id),
            &mut files,
        )
        .await?;
        collect_tree(
            config_dir,
            config_dir.join("tasks").join(&session_id),
            &mut files,
        )
        .await?;

        Ok(Continuation { session_id, files })
    }
}

/// Collect one file, if it exists and is UTF-8.
async fn collect_file(
    config_dir: &Path,
    path: &Path,
    files: &mut Vec<ContinuationFile>,
) -> io::Result<()> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    // Not UTF-8 is not carried — the token holds text.
    let Ok(content) = String::from_utf8(bytes) else {
        return Ok(());
    };
    let Ok(relative) = path.strip_prefix(config_dir) else {
        return Ok(());
    };
    files.push(ContinuationFile {
        path: relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => part.to_str(),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/"),
        content,
    });
    Ok(())
}

/// Collect every file under `root`, walked with an explicit stack —
/// absence contributes nothing.
async fn collect_tree(
    config_dir: &Path,
    root: std::path::PathBuf,
    files: &mut Vec<ContinuationFile>,
) -> io::Result<()> {
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                continue;
            }
            Err(error) => return Err(error),
        };
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                stack.push(entry.path());
            } else {
                collect_file(config_dir, &entry.path(), files).await?;
            }
        }
    }
    Ok(())
}
