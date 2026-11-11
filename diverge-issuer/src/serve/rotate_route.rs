//! The HTTP rotation: `POST /rotate`, from loopback only.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Json;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use super::Issuer;
use crate::signing;

/// What the rotation is asked.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct RotateQuery {
    /// Revoke every refresh chain as well. Absent, `false`.
    #[serde(default)]
    pub sign_out: bool,
}

/// `POST /rotate?sign_out=…`: the issuer rotates its signing key
/// itself — see [`signing::rotate`]. The rules:
///
/// - Only a loopback peer is heard, as at `/stop`: any other is
///   answered `403 Forbidden` with nothing done. Loopback is the whole
///   credential.
/// - The live keys' write lock is taken first and held to the end, so
///   no token is signed or verified while the key changes.
/// - A rotation done is answered `200 OK` with
///   `{"kid","made","removed","signed_out"}`, and the issuer signs and
///   publishes the new key from the next request on.
/// - A rotation that failed changed nothing, in the database or in
///   memory, and is answered `500` with `{"error"}`.
pub async fn rotate(
    State(issuer): State<Arc<Issuer>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(query): Query<RotateQuery>,
) -> Response {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut keys = issuer.keys.write().await;
    match signing::rotate(&issuer.store, query.sign_out).await {
        Ok((rotated, live)) => {
            *keys = live;
            let body = json!({
                "kid": rotated.key.kid,
                "made": rotated.key.made,
                "removed": rotated.removed,
                "signed_out": rotated.signed_out,
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": error.to_string() }))).into_response(),
    }
}
