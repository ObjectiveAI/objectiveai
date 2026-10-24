//! An OAuth error, as an endpoint answers one.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::store;

/// An error answered as RFC 6749 §5.2 answers one: a status, and a
/// JSON body `{"error", "error_description"}`, never cached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthError {
    /// The HTTP status.
    pub status: StatusCode,
    /// The error code, as the RFCs name it.
    pub code: &'static str,
    /// What went wrong, for the person reading the client's log.
    pub description: String,
    /// Whether the client authenticated with `Basic`, which a
    /// `401 Unauthorized` names back in `WWW-Authenticate`, as RFC 6749
    /// §5.2 requires.
    pub basic: bool,
}

impl OAuthError {
    /// An error with status `400 Bad Request`.
    pub fn bad(code: &'static str, description: impl Into<String>) -> OAuthError {
        OAuthError { status: StatusCode::BAD_REQUEST, code, description: description.into(), basic: false }
    }

    /// `invalid_request`: the request is missing a parameter, or has
    /// one it may not.
    pub fn invalid_request(description: impl Into<String>) -> OAuthError {
        OAuthError::bad("invalid_request", description)
    }

    /// `invalid_client`: the client could not be authenticated, with
    /// status `401 Unauthorized`.
    pub fn invalid_client(basic: bool) -> OAuthError {
        OAuthError {
            status: StatusCode::UNAUTHORIZED,
            code: "invalid_client",
            description: "the client could not be authenticated".to_string(),
            basic,
        }
    }

    /// `invalid_grant`: the device code or the refresh token is not
    /// good.
    pub fn invalid_grant(description: impl Into<String>) -> OAuthError {
        OAuthError::bad("invalid_grant", description)
    }

    /// `server_error`: the issuer failed, with status
    /// `500 Internal Server Error`.
    pub fn server(error: impl std::fmt::Display) -> OAuthError {
        OAuthError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "server_error",
            description: error.to_string(),
            basic: false,
        }
    }
}

impl From<store::Error> for OAuthError {
    fn from(error: store::Error) -> Self {
        OAuthError::server(error)
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.code, "error_description": self.description }));
        let mut response = (self.status, no_store(), body).into_response();
        if self.basic && self.status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic realm=\"diverge-issuer\""));
        }
        response
    }
}

/// The headers every answer that carries a token or an error is sent
/// with, as RFC 6749 §5.1 requires: never cached.
pub fn no_store() -> [(header::HeaderName, &'static str); 2] {
    [(header::CACHE_CONTROL, "no-store"), (header::PRAGMA, "no-cache")]
}
