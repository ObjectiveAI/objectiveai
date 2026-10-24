//! `GET /jwks`: the key set.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;

use crate::serve::Issuer;

/// How long a verifier may keep the key set, in seconds: short, so
/// that a key a rotation retired stops verifying soon after the
/// issuer's restart.
const MAX_AGE: &str = "max-age=60";

/// `GET /jwks`: every unretired signing key's public half, as the
/// JSON Web Key Set RFC 7517 §5 defines, made at the start — see
/// [`signing`](crate::signing) — to be kept a minute at most.
pub async fn jwks(State(issuer): State<Arc<Issuer>>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, MAX_AGE)], issuer.key_set.clone())
}
