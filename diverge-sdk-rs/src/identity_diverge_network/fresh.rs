//! An account's access token, good now.

use super::device::{Answer, post};
use super::login::now;
use super::pending::Tokens;
use super::provider::Provider;
use super::record::Record;
use super::{Account, Accounts, Error};

/// The seconds before its expiry an access token is renewed at: one
/// that would end while a request is on its way is not used.
const EARLY: i64 = 60;

impl Accounts {
    /// `record`'s access token, renewed first where it is about to
    /// expire. The rules:
    ///
    /// 1. A token with no expiry — GitHub's — is answered as it is.
    /// 2. One good for longer than [`EARLY`] is answered as it is.
    /// 3. Any other is renewed: the refresh token spent at the
    ///    provider's token endpoint, RFC 6749 §6, as the client
    ///    `identity.diverge.network` is there. The new access token,
    ///    its expiry, and the ID token and the refresh token where the
    ///    provider answered new ones, are written to the account's
    ///    file before the token is answered.
    /// 4. An account with no access token, one whose token expired
    ///    with no refresh token to renew it by, and a renewal the
    ///    provider refuses, are [`Error::SignedOut`].
    pub(super) async fn access(&self, provider: &Provider, record: &mut Record) -> Result<String, Error> {
        let signed_out = |record: &Record| Error::SignedOut(Account { provider: record.provider.clone(), id: record.id.clone() });
        // Rules 1 and 2.
        let good = record.access_expires.is_none_or(|expires| expires > now().saturating_add(EARLY));
        if good {
            return record.access_token.clone().ok_or_else(|| signed_out(record));
        }
        // Rule 3.
        let refresh_token = record.refresh_token.clone().ok_or_else(|| signed_out(record))?;
        let mut form = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str()), ("client_id", provider.client_id)];
        if let Some(secret) = provider.client_secret {
            form.push(("client_secret", secret));
        }
        let tokens = match post::<Tokens>(&self.http, provider.token_endpoint, &form).await? {
            Answer::Answered(tokens) => tokens,
            // Rule 4.
            Answer::Refused { .. } => return Err(signed_out(record)),
        };
        let access_token = tokens.access_token.ok_or_else(|| signed_out(record))?;
        record.access_token = Some(access_token.clone());
        record.access_expires = tokens.expires_in.map(|seconds| now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX)));
        if let Some(id_token) = tokens.id_token {
            record.id_token = Some(id_token);
        }
        if let Some(refresh_token) = tokens.refresh_token {
            record.refresh_token = Some(refresh_token);
        }
        self.write(record).await?;
        Ok(access_token)
    }
}
