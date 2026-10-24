//! `GET /jwks`: the key set.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;

use crate::serve::Issuer;

/// `GET /jwks`: every unretired signing key's public half, as the
/// JSON Web Key Set RFC 7517 §5 defines, made at the start — see
/// [`signing`](crate::signing).
pub async fn jwks(State(issuer): State<Arc<Issuer>>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], issuer.key_set.clone())
}
