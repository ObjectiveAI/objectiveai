//! The issuer's one app.

use axum::http::{HeaderMap, header};

use super::OAuthError;

/// The id of the one client the issuer serves: every Diverge program
/// signs in as it — the CLI, the viewer, a daemon, a provider — as a
/// public client, with no secret.
pub const CLIENT_ID: &str = "diverge";

/// Whether a request at the token, device authorization or revocation
/// endpoint is from the issuer's one app. The rules:
///
/// 1. The form's `client_id` is [`CLIENT_ID`], exactly.
/// 2. The app is public: a `client_secret`, and an `Authorization`
///    header of any scheme, are refused.
///
/// Anything else is `invalid_client`, with `401 Unauthorized`.
pub fn client(headers: &HeaderMap, client_id: Option<&str>, client_secret: Option<&str>) -> Result<(), OAuthError> {
    let basic = headers.contains_key(header::AUTHORIZATION);
    if basic || client_secret.is_some() || client_id != Some(CLIENT_ID) {
        return Err(OAuthError::invalid_client(basic));
    }
    Ok(())
}
