//! Error types for the ObjectiveAI SDK.
//!
//! This module provides error handling utilities for API responses,
//! including a trait for status-based errors and a concrete error type
//! for API response errors.

use serde::{Deserialize, Serialize};

/// A trait for errors that have an HTTP status code and optional message.
///
/// This trait is implemented by error types that represent API responses
/// with a status code and message body.
pub trait StatusError {
    /// Returns the HTTP status code associated with this error.
    fn status(&self) -> u16;

    /// Returns the error message, if any.
    ///
    /// The message is returned as a JSON value to accommodate both
    /// simple string messages and structured error details.
    fn message(&self) -> Option<serde_json::Value>;
}

/// An error returned by the ObjectiveAI API.
///
/// This struct represents an API error response containing an HTTP status
/// code and a message. The message can be any JSON value, allowing for
/// both simple string errors and structured error objects.
///
/// # Examples
///
/// ```
/// use objectiveai::error::ResponseError;
/// use serde_json::json;
///
/// let error = ResponseError {
///     code: 400,
///     message: json!({"error": "Invalid request"}),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
pub struct ResponseError {
    /// The HTTP status code of the error response.
    pub code: u16,
    /// The error message or details as a JSON value.
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
