//! Why a watch could not be made, or died.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_provider_sdk::shared::error;
use serde_json::json;
use tokio::task::JoinError;

/// What [`watch`](super::watch) fails with, and what a [`Watch`](super::Watch)
/// yields once before it ends.
#[derive(Debug)]
pub enum Error {
    /// The root is not a directory that exists.
    Root(PathBuf),
    /// The watcher could not be made, or could not watch the root.
    Watch(notify::Error),
    /// The root could not be made canonical.
    Io(io::Error),
    /// A blocking task — the walk, an event's mapping — died.
    Walk(JoinError),
    /// The watcher stopped on its own: its thread is gone.
    Stopped,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Root(path) => write!(f, "`{}` is not a directory", path.display()),
            Error::Watch(error) => write!(f, "the directory could not be watched: {error}"),
            Error::Io(error) => write!(f, "the root could not be resolved: {error}"),
            Error::Walk(error) => write!(f, "the walk died: {error}"),
            Error::Stopped => f.write_str("the watcher stopped"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Root(_) => None,
            Error::Watch(error) => Some(error),
            Error::Io(error) => Some(error),
            Error::Walk(error) => Some(error),
            Error::Stopped => None,
        }
    }
}

/// What the SDK puts on the wire for one of these.
///
/// ```json
/// {"kind":"watch","error":"the directory could not be watched: …"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        error::Error(json!({
            "kind": "watch",
            "error": error.to_string(),
        }))
    }
}
