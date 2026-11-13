//! What can go wrong in one run of the harness.

/// A run that produced no turn.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feed would not serialize — plain data never fails to, so
    /// this names a bug.
    #[error("the feed would not serialize: {0}")]
    Feed(serde_json::Error),
    /// No agent has been registered, so there is no bundle to run.
    #[error("no agent has been registered")]
    Unregistered,
    /// `lone` could not be started.
    #[error("lone could not be started: {0}")]
    Spawn(std::io::Error),
    /// The process could not be waited for.
    #[error("the harness process could not be waited for: {0}")]
    Wait(std::io::Error),
    /// The turn raised — or was killed. stderr is the reason.
    #[error("the harness raised")]
    Exception {
        /// The exit code, absent if a signal ended the process.
        status: Option<i32>,
        /// Everything on stderr, the reason among it.
        stderr: String,
    },
    /// The process ended cleanly and its last line is not the
    /// envelope: something broke the contract.
    #[error("the harness produced no envelope: {error}")]
    Harness {
        /// The last non-empty line of stdout, as found.
        line: String,
        /// Why it is not an envelope.
        error: serde_json::Error,
    },
    /// `eval` was `null` and what was printed is not JSON.
    #[error("what the harness printed is not JSON: {0}")]
    Printed(serde_json::Error),
    /// `eval` was `null` and nothing was printed.
    #[error("the harness produced no output")]
    NoOutput,
    /// The value is not an array of chunks.
    #[error("the output is not a list of chunks: {0}")]
    Deserialize(serde_path_to_error::Error<serde_json::Error>),
    /// A chunk the harness may not say.
    #[error("the output's chunk {index} is a `{kind}`, which the harness may not say")]
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
            Error::Unregistered => serde_json::json!({
                "kind": "unregistered",
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
            "kind": "lone",
            "error": error,
        })
    }
}
