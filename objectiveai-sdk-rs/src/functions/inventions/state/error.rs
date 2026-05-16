#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing required file: {0}")]
    MissingFile(&'static str),
    #[error("deserialization error in {file}: {source}")]
    Deserialize {
        file: &'static str,
        source: serde_path_to_error::Error<serde_json::Error>,
    },
    #[error("function.json contains unrecognized function type: {0}")]
    UnrecognizedFunctionType(String),
}
