//! Core API error type.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A trait for errors that have an HTTP status code and optional message.
pub trait StatusError {
    /// Returns the HTTP status code associated with this error.
    fn status(&self) -> u16;

    /// Returns the error message, if any.
    fn message(&self) -> Option<serde_json::Value>;
}

/// An error returned by the ObjectiveAI API.
///
/// This struct represents an API error response containing an HTTP status
/// code and a message. The message can be any JSON value, allowing for
/// both simple string errors and structured error objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error, JsonSchema, arbitrary::Arbitrary)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
#[schemars(rename = "error.ResponseError")]
pub struct ResponseError {
    /// The HTTP status code of the error response.
    pub code: u16,
    /// The error message or details as a JSON value.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_json_value)]
    pub message: serde_json::Value,
}

impl StatusError for ResponseError {
    fn status(&self) -> u16 {
        self.code
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(self.message.clone())
    }
}

impl<T> From<&T> for ResponseError
where
    T: StatusError,
{
    fn from(error: &T) -> Self {
        ResponseError {
            code: error.status(),
            message: error.message().unwrap_or_default(),
        }
    }
}
