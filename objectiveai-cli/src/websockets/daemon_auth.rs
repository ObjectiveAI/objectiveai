//! Optional auth for the daemon's broadcast WebSocket server.
//!
//! Identical strategy to the viewer's (`objectiveai-viewer`'s
//! `signature.rs`): a client authenticates by sending one of
//! `X-DAEMON-SIGNATURE` / `DAEMON-SIGNATURE` / `X-OBJECTIVEAI-SIGNATURE`
//! / `OBJECTIVEAI-SIGNATURE` containing `sha256=<hex>`, where `<hex>` is
//! `SHA256(secret)`. The middleware short-circuits with `401` when the
//! signature is missing or doesn't match. Knowing the signature does not
//! reveal the secret (preimage resistance).
//!
//! Optional: when the daemon's `DAEMON_SECRET` is unset the middleware is
//! never layered (open server); this module only runs when a secret is
//! present, so `secret` here is always `Some`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use subtle::ConstantTimeEq;

/// Reject the WebSocket-upgrade request with `401` unless it carries a
/// signature header matching `sha256=<hex(SHA256(secret))>`. The upgrade
/// is a bodyless GET, so — unlike the viewer's HTTP middleware — there's
/// no request body to buffer and rebuild; the headers are inspected in
/// place and the request passes through untouched on success.
pub(crate) async fn signature_middleware(
    State(secret): State<Option<Arc<String>>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let Some(secret) = &secret else {
        // Should not happen (the layer is only added when a secret is
        // configured), but treat a missing secret as an open server.
        return Ok(next.run(request).await);
    };
    let headers = request.headers();
    let signature = headers
        .get("X-DAEMON-SIGNATURE")
        .or_else(|| headers.get("DAEMON-SIGNATURE"))
        .or_else(|| headers.get("X-OBJECTIVEAI-SIGNATURE"))
        .or_else(|| headers.get("OBJECTIVEAI-SIGNATURE"))
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !verify_signature(secret, signature) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(request).await)
}

/// `true` iff `signature_header` is `sha256=<hex(SHA256(secret))>`. The
/// signature is a static, pre-computed value; the comparison is
/// constant-time to avoid leaking it. Identical math to the viewer's
/// `verify_signature` and `generate_viewer_secret_signature_pair`.
fn verify_signature(secret: &str, signature_header: &str) -> bool {
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    use sha2::{Digest, Sha256};
    let expected = Sha256::digest(secret.as_bytes());
    expected.ct_eq(&sig_bytes).into()
}
