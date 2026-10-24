//! A client to add, and the rules it is held to.

use url::Url;

use super::{Grant, Kind};
use crate::store::{Error, names};

/// A client as it is added — see [`add`](super::Clients::add).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct New {
    /// The client's id.
    pub id: String,
    /// Whether it has a secret.
    pub kind: Kind,
    /// The SHA-256 of its secret as lowercase hex, for a confidential
    /// client; `None` for a public one.
    pub secret_hash: Option<String>,
    /// The grants it may use.
    pub grants: Vec<Grant>,
    /// The audiences its access tokens may be for, in order.
    pub audiences: Vec<String>,
    /// The URIs the authorization code may be sent to.
    pub redirects: Vec<String>,
}

impl New {
    /// Whether the client may be added as it is put. The rules, each
    /// refused as [`Error::Invalid`]:
    ///
    /// 1. The id is a client id — see [`names::client`].
    /// 2. A confidential client has a secret's hash, and a public one
    ///    has none.
    /// 3. It has at least one grant, and at least one audience, none
    ///    named twice.
    /// 4. A public client does not have `client_credentials`: that
    ///    grant is a secret's alone.
    /// 5. `refresh_token` comes with `authorization_code` or
    ///    `device_code`, the grants a refresh token is issued by.
    /// 6. It has a redirect URI exactly when it has
    ///    `authorization_code`, and every one is an absolute URL with
    ///    no fragment.
    pub fn check(&self) -> Result<(), Error> {
        let refuse = |why: &str| Err(Error::Invalid(format!("the client `{}` {why}", self.id)));
        names::client(&self.id)?;
        if (self.kind == Kind::Confidential) != self.secret_hash.is_some() {
            return refuse("has a secret exactly when it is confidential");
        }
        if self.grants.is_empty() {
            return refuse("needs at least one grant");
        }
        if self.audiences.is_empty() {
            return refuse("needs at least one audience");
        }
        if twice(&self.grants) || twice(&self.audiences) || twice(&self.redirects) {
            return refuse("names a grant, an audience or a redirect URI twice");
        }
        let has = |grant| self.grants.contains(&grant);
        if self.kind == Kind::Public && has(Grant::ClientCredentials) {
            return refuse("is public, and `client_credentials` needs a secret");
        }
        if has(Grant::RefreshToken) && !has(Grant::AuthorizationCode) && !has(Grant::DeviceCode) {
            return refuse("has `refresh_token`, which needs `authorization_code` or `device_code`");
        }
        if has(Grant::AuthorizationCode) == self.redirects.is_empty() {
            return refuse("has redirect URIs exactly when it has `authorization_code`");
        }
        for redirect in &self.redirects {
            match Url::parse(redirect) {
                Ok(url) if url.fragment().is_none() => {}
                _ => return refuse("has a redirect URI that is not an absolute URL without a fragment"),
            }
        }
        Ok(())
    }
}

/// Whether any value of `values` is in it twice.
fn twice<T: PartialEq>(values: &[T]) -> bool {
    values.iter().enumerate().any(|(index, value)| values[..index].contains(value))
}
