//! The `client_credentials` grant.

use axum::response::Response;

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::{OAuthError, audience, scope};
use crate::serve::Issuer;
use crate::store::clients::{Client, Grant};

/// Tokens for the client itself, OAuth 2.0 §4.4: an access token whose
/// subject is the client's id, for the scope and the audience asked
/// for. No identity signed in, so there is no ID token; and the client
/// holds its secret, so there is no refresh token, as §4.4.3 advises.
pub async fn client_credentials(issuer: &Issuer, client: &Client, form: &TokenForm) -> Result<Response, OAuthError> {
    if !client.may(Grant::ClientCredentials) {
        return Err(OAuthError::unauthorized_client());
    }
    let grantee = Grantee {
        sub: client.id.clone(),
        scope: scope(form.scope.as_deref()),
        audience: audience(client, form.resource.as_deref())?,
        auth_time: None,
        nonce: None,
    };
    answer(issuer, client, grantee, Next::None).await
}
