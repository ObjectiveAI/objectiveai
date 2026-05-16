//! Auth middleware for the viewer's embedded axum server.
//!
//! Clients that authenticate to the viewer's HTTP endpoint send one
//! of `X-VIEWER-SIGNATURE` / `VIEWER-SIGNATURE` /
//! `X-OBJECTIVEAI-SIGNATURE` / `OBJECTIVEAI-SIGNATURE` containing
//! `sha256=<hex>` where `<hex>` is `SHA256(secret)`. The middleware
//! short-circuits with `401` when the signature is missing or
//! doesn't match. When the viewer's `secret` config is `None`, the
//! middleware is a no-op (open server).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use subtle::ConstantTimeEq;

pub(crate) async fn signature_middleware(
    State(secret): State<Option<Arc<String>>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(secret) = &secret {
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let headers = &parts.headers;
        let signature = headers
            .get("X-VIEWER-SIGNATURE")
            .or_else(|| headers.get("VIEWER-SIGNATURE"))
            .or_else(|| headers.get("X-OBJECTIVEAI-SIGNATURE"))
            .or_else(|| headers.get("OBJECTIVEAI-SIGNATURE"))
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if !verify_signature(secret, &bytes, signature) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let rebuilt = axum::http::Request::from_parts(parts, axum::body::Body::from(bytes));
        Ok(next.run(rebuilt).await)
    } else {
        Ok(next.run(request).await)
    }
}

fn verify_signature(secret: &str, _body: &[u8], signature_header: &str) -> bool {
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    // Compute SHA256(secret) and compare against the provided signature.
    // The signature is a static pre-computed value: sha256=<SHA256(secret)>.
    // Knowing the signature does not reveal the secret (preimage resistance).
    use sha2::{Digest, Sha256};
    let expected = Sha256::digest(secret.as_bytes());
    expected.ct_eq(&sig_bytes).into()
}
