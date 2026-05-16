#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("deserialization error: {0}")]
    DeserializationError(#[from] serde_path_to_error::Error<serde_json::Error>),
    #[error("received bad status code: {code}, body: {body}")]
    BadStatus {
        code: reqwest::StatusCode,
        body: serde_json::Value,
    },
    #[error("fetch request error: {0}")]
    RequestError(reqwest::Error),
    #[error("fetch response error: {0}")]
    ResponseError(reqwest::Error),
    #[error("errors: {0}, {1}")]
    MultipleErrors(Box<Error>, Box<Error>),
    #[error("profile commit SHA is required")]
    ProfileCommitShaRequired,
    #[error("no publish GitHub token available")]
    MissingPublishToken,
    #[error("filesystem error: {0}")]
    Filesystem(#[from] crate::filesystem::Error),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::DeserializationError(_) => 400,
            Error::BadStatus { code, .. } => code.as_u16(),
            Error::RequestError(_) => 500,
            Error::ResponseError(_) => 500,
            Error::MultipleErrors(e1, e2) => {
                let status2 = e2.status();
                if status2 != 500 { status2 } else { e1.status() }
            }
            Error::ProfileCommitShaRequired => 400,
            Error::MissingPublishToken => 400,
            Error::Filesystem(e) => e.status(),
            Error::Join(_) => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "github",
            "error": match self {
                Error::DeserializationError(e) => serde_json::json!({
                    "kind": "deserialization",
                    "error": e.to_string(),
                }),
                Error::BadStatus { body, .. } => serde_json::json!({
                    "kind": "bad_status",
                    "error": body,
                }),
                Error::RequestError(e) => serde_json::json!({
                    "kind": "request",
                    "error": e.to_string(),
                }),
                Error::ResponseError(e) => serde_json::json!({
                    "kind": "response",
                    "error": e.to_string(),
                }),
                Error::MultipleErrors(e1, e2) => serde_json::json!({
                    "kind": "multiple",
                    "error_1": e1.message(),
                    "error_2": e2.message(),
                }),
                Error::ProfileCommitShaRequired => serde_json::json!({
                    "kind": "profile_commit_sha_required",
                    "error": "Profile commit SHA is required for remote function tasks.",
                }),
                Error::MissingPublishToken => serde_json::json!({
                    "kind": "missing_publish_token",
                    "error": "No publish GitHub token available. Provide a github_token in the request or configure PUBLISH_GITHUB_TOKEN on the server.",
                }),
                Error::Filesystem(e) => serde_json::json!({
                    "kind": "filesystem",
                    "error": e.message(),
                }),
                Error::Join(e) => serde_json::json!({
                    "kind": "join",
                    "error": e.to_string(),
                }),
            }
        }))
    }
}
