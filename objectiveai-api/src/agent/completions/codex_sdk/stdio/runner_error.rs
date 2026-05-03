use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("runner subprocess has exited")]
    Closed,
    #[error("spawn runner: {0}")]
    Spawn(String),
    #[error("write to runner stdin: {0}")]
    Write(String),
    #[error("serialize runner request: {0}")]
    Serialize(#[from] serde_json::Error),
    /// A request id was registered but the registry already had one
    /// with the same id. The caller's id-allocator is broken.
    #[error("duplicate request id: {0}")]
    DuplicateId(String),
}
