//! The device code grant, RFC 8628 §3.4.

use axum::response::Response;

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::OAuthError;
use crate::serve::Issuer;
use crate::store::clients::{Client, Grant};
use crate::store::devices::Poll;
use crate::tokens;

/// Poll the device code the form names. The form has `device_code`,
/// else `invalid_request`. What the poll finds — see
/// [`devices`](crate::store::devices) — is answered as RFC 8628 §3.5
/// states:
///
/// | the poll finds | the answer |
/// |----------------|------------|
/// | no such code for this client | `invalid_grant` |
/// | the code expired | `expired_token` |
/// | no decision yet | `authorization_pending` |
/// | a poll too soon | `slow_down` |
/// | a denial | `access_denied` |
/// | an approval | the tokens, as for the authorization code |
pub async fn device_code(issuer: &Issuer, client: &Client, form: &TokenForm) -> Result<Response, OAuthError> {
    if !client.may(Grant::DeviceCode) {
        return Err(OAuthError::unauthorized_client());
    }
    let device_code = form.device_code.as_deref().ok_or_else(|| OAuthError::invalid_request("`device_code` is required"))?;
    let approved = match issuer.store.devices().poll(&tokens::hash(device_code), &client.id).await? {
        Poll::Unknown => return Err(OAuthError::invalid_grant("the device code is not one this client holds")),
        Poll::Expired => return Err(OAuthError::bad("expired_token", "the device code has expired")),
        Poll::Pending => return Err(OAuthError::bad("authorization_pending", "no one has approved the code yet")),
        Poll::SlowDown => return Err(OAuthError::bad("slow_down", "the poll came sooner than the interval")),
        Poll::Denied => return Err(OAuthError::bad("access_denied", "the code was denied")),
        Poll::Approved(approved) => approved,
    };
    let grantee = Grantee {
        sub: approved.subject,
        scope: approved.scope,
        audience: approved.audience,
        auth_time: Some(approved.auth_time),
        nonce: None,
    };
    let next = if client.may(Grant::RefreshToken) { Next::Begin } else { Next::None };
    answer(issuer, client, grantee, next).await
}
