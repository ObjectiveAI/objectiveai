//! Optional auth for the daemon's routes — two mechanisms, one policy.
//!
//! - **HTTP routes** (everything except `/laboratory`): the
//!   `X-OBJECTIVEAI-SIGNATURE` request header, checked by
//!   [`authenticate_header`]. `401` on a missing/invalid signature.
//!   The `/mcp` routes included — MCP clients configure the
//!   `X-OBJECTIVEAI-SIGNATURE` header like any other consumer.
//! - **The `/laboratory` WebSocket** (the daemon's ONE remaining WS —
//!   the bidirectional host channel): a first-message text-frame
//!   preamble, the SDK [`AuthEnvelope`] —
//!   `{"signature": "sha256=<hex>"}` where `<hex>` is
//!   `SHA256(secret)`, or `{"signature": null}` when the client has
//!   none — checked by [`authenticate`] (demoted to the SECOND frame
//!   there: identity precedes authorization).
//!
//! Both apply the same policy: secret configured ⇒ a missing/invalid
//! signature is rejected; no secret ⇒ the credential is consumed and
//! ignored. Knowing the signature does not reveal the secret
//! (preimage resistance).

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use objectiveai_sdk::daemon::AuthEnvelope;
use subtle::ConstantTimeEq;

/// Consume the connection's auth preamble: read frames until the
/// first text frame (control frames are ignored), parse it as an
/// [`AuthEnvelope`], and verify it against `secret` when one is
/// configured. Returns `true` when the connection may proceed; on any
/// failure — client gone, unparseable preamble, missing or invalid
/// signature — the socket is closed and `false` returned.
pub(crate) async fn authenticate(socket: &mut WebSocket, secret: Option<&Arc<String>>) -> bool {
    let text = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => break text,
            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return false,
            Some(Ok(_)) => continue,
        }
    };
    let Ok(envelope) = serde_json::from_str::<AuthEnvelope>(&text) else {
        let _ = socket.send(Message::Close(None)).await;
        return false;
    };
    if let Some(secret) = secret {
        let verified = envelope
            .signature
            .as_deref()
            .is_some_and(|signature| verify_signature(secret, signature));
        if !verified {
            let _ = socket.send(Message::Close(None)).await;
            return false;
        }
    }
    true
}

/// Header-based auth for the daemon's HTTP routes (`/execute`,
/// `/listen`, `/laboratories/*`, `/agents/instances/*`). Reads the
/// `X-OBJECTIVEAI-SIGNATURE` header and applies the same policy as
/// [`authenticate`]: when a `secret` is configured the header must be
/// present and valid; without one, any header is ignored.
pub(crate) fn authenticate_header(
    headers: &axum::http::HeaderMap,
    secret: Option<&Arc<String>>,
) -> bool {
    let Some(secret) = secret else {
        return true;
    };
    headers
        .get("X-OBJECTIVEAI-SIGNATURE")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|signature| verify_signature(secret, signature))
}

/// `true` iff `signature` is `sha256=<hex(SHA256(secret))>`. The
/// signature is a static, pre-computed value; the comparison is
/// constant-time to avoid leaking it. Identical math to the cli's
/// `generate_viewer_secret_signature_pair`.
fn verify_signature(secret: &str, signature: &str) -> bool {
    let Some(hex_sig) = signature.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    use sha2::{Digest, Sha256};
    let expected = Sha256::digest(secret.as_bytes());
    expected.ct_eq(&sig_bytes).into()
}

/// The client-side half of [`verify_signature`]'s math:
/// `sha256=<hex(SHA256(secret))>` — identical to
/// `generate_viewer_secret_signature_pair`'s derivation. Used when the
/// CLI launches a consumer (the laboratory manager) that must present
/// the signature back to this daemon.
pub fn derive_signature(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(secret.as_bytes());
    format!("sha256={}", hex::encode(digest))
}
