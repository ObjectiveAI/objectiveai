//! `GET`, `POST /authorize`: refused.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// The authorization endpoint OpenID Connect Discovery requires a
/// provider to name, answered with `unsupported_response_type`: the
/// issuer's one app signs in by the device code alone, so no browser is
/// ever sent here and back.
pub async fn authorize() -> Response {
    let body = json!({
        "error": "unsupported_response_type",
        "error_description": "this issuer signs in by the device authorization grant alone",
    });
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}
