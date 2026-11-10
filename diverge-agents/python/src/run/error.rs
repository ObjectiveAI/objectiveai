//! What can go wrong in one run of the script.

/// A run that produced no turn.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feed would not serialize — plain data never fails to, so
    /// this names a bug.
    #[error("the feed would not serialize: {0}")]
    Feed(serde_json::Error),
    /// `python3` could not be started.
    #[error("python3 could not be started: {0}")]
    Spawn(std::io::Error),
    /// The process could not be waited for.
    #[error("the python process could not be waited for: {0}")]
    Wait(std::io::Error),
    /// The script raised — or was killed. stderr is the traceback.
    #[error("the script raised")]
    Exception {
        /// The exit code, absent if a signal ended the process.
        status: Option<i32>,
        /// Everything on stderr, the traceback among it.
        stderr: String,
    },
    /// The process ended cleanly and its last line is not the
    /// envelope: the script broke the harness.
    #[error("the harness produced no envelope: {error}")]
    Harness {
        /// The last non-empty line of stdout, as found.
        line: String,
        /// Why it is not an envelope.
        error: serde_json::Error,
    },
    /// The last expression was `None` and what the script printed is
    /// not JSON.
    #[error("what the script printed is not JSON: {0}")]
    Printed(serde_json::Error),
    /// The last expression was `None` and the script printed nothing.
    #[error("the script produced no output")]
    NoOutput,
    /// The value is not an array of chunks.
    #[error("the output is not a list of chunks: {0}")]
    Deserialize(serde_path_to_error::Error<serde_json::Error>),
    /// A chunk the script may not say.
    #[error("the output's chunk {index} is a `{kind}`, which a script may not say")]
    Forbidden {
        /// Where in the array.
        index: usize,
        /// Which forbidden kind.
        kind: &'static str,
    },
}

impl Error {
    /// The failure as JSON — a notification's message, or the error
    /// frame's, depending on whether the loop had already spoken.
    pub fn message(&self) -> serde_json::Value {
        let error = match self {
            Error::Feed(_) => serde_json::json!({
                "kind": "feed",
                "error": self.to_string(),
            }),
            Error::Spawn(_) => serde_json::json!({
                "kind": "spawn",
                "error": self.to_string(),
            }),
            Error::Wait(_) => serde_json::json!({
                "kind": "wait",
                "error": self.to_string(),
            }),
            Error::Exception { status, stderr } => serde_json::json!({
                "kind": "exception",
                "status": status,
                "stderr": stderr,
            }),
            Error::Harness { line, error } => serde_json::json!({
                "kind": "harness",
                "line": line,
                "error": error.to_string(),
            }),
            Error::Printed(_) => serde_json::json!({
                "kind": "printed",
                "error": self.to_string(),
            }),
            Error::NoOutput => serde_json::json!({
                "kind": "no_output",
                "error": self.to_string(),
            }),
            Error::Deserialize(error) => serde_json::json!({
                "kind": "deserialize",
                "path": error.path().to_string(),
                "error": error.inner().to_string(),
            }),
            Error::Forbidden { index, kind } => serde_json::json!({
                "kind": "forbidden",
                "index": index,
                "chunk": kind,
            }),
        };
        serde_json::json!({
            "kind": "python",
            "error": error,
        })
    }
}
