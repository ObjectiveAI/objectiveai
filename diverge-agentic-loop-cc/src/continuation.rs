//! What a ClaudeCode continuation token holds: a filesystem.
//!
//! Claude Code keeps its conversation on disk — append-only JSONL
//! under `$CLAUDE_CONFIG_DIR/projects/<sanitized-cwd>/<session>.jsonl`
//! plus its adjacent files, with no locks and no indexes (see
//! `cc-source-findings/HISTORY.md`). Resuming is those files existing
//! and `claude --resume <session id>` being launched with the same
//! working directory. So this container's continuation IS the
//! files: the token carries the session id and everything needed to
//! make it resumable, and running a continuation means writing them
//! back before Claude Code starts.
//!
//! The container fixes its geometry — one working directory, one
//! config dir (the stock `~/.claude`, no environment overrides) — so
//! every relative path in a token minted by one run resolves
//! identically in the next. The harness's flow:
//! fresh runs launch Claude Code and harvest `projects/**` into a
//! token; resumed runs [`write`](Continuation::write) the token's
//! files and launch with `--resume` and the token's session id.

use std::io;
use std::path::{Component, Path};

/// Where the session state lives, fixed for the container's life:
/// Claude Code's own default, `~/.claude` for the container's root
/// user. The environment stays stock — no `CLAUDE_CONFIG_DIR` — so
/// what [`Continuation::write`] lays down is what Claude Code finds.
pub const CONFIG_DIR: &str = "/root/.claude";

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// A continuation token, opened.
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
    /// Open a raw continuation string: un-base64 it, deserialize it.
    pub fn parse(token: &str) -> Result<Self, ContinuationError> {
        let json = STANDARD
            .decode(token)
            .map_err(ContinuationError::Base64)?;
        serde_json::from_slice(&json).map_err(ContinuationError::Json)
    }

    /// Close the coat back up: serialize the state, base64 it.
    /// [`parse`](Self::parse)'s exact inverse.
    pub fn tokenize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_vec(self).map(|json| STANDARD.encode(json))
    }

    /// Write every file under [`CONFIG_DIR`], parents created as
    /// needed. Safe to do wholesale before Claude Code starts:
    /// nothing else is reading, and the format has no index or lock
    /// to maintain.
    ///
    /// A path that is absolute, names a root, or contains a `..`
    /// component is refused with [`io::ErrorKind::InvalidInput`] —
    /// the token names files inside the config directory, and does
    /// not get to escape it.
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
}

/// A continuation token that could not be opened.
///
/// Two layers, two failures: the coat did not decode, or what was
/// inside was not the state this container writes.
#[derive(Debug)]
pub enum ContinuationError {
    /// The token is not base64.
    Base64(base64::DecodeError),
    /// The decoded bytes are not the session state.
    Json(serde_json::Error),
}

impl std::fmt::Display for ContinuationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContinuationError::Base64(error) => {
                write!(f, "a continuation token is not base64: {error}")
            }
            ContinuationError::Json(error) => {
                write!(f, "a continuation token did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for ContinuationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContinuationError::Base64(error) => Some(error),
            ContinuationError::Json(error) => Some(error),
        }
    }
}
