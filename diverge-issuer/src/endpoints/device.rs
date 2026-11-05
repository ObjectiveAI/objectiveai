//! `GET /device`: the verification URI, which signs no one in.

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// What the page says: who approves a code, and how.
const SAYS: &str = "This issuer has no sign-in page.\n\nIts operator approves a code on the issuer's own host:\n\n    diverge issuer pending\n    diverge issuer approve <code>\n";

/// `GET /device`: the verification URI RFC 8628 requires an answer to
/// name, as plain text saying that the issuer's operator approves a
/// code on its host. It reads nothing and decides nothing.
pub async fn device_page() -> Response {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], SAYS).into_response()
}
