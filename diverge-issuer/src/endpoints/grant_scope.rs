//! A request's scope and audience.

use super::OAuthError;
use crate::store::clients::Client;

/// `OpenID Connect`'s scope: an ID token is issued exactly when the
/// scope holds it.
pub const OPENID: &str = "openid";

/// The scope a request asks for, as OAuth writes it: its values
/// separated by single spaces, each once, in the order first given.
/// No value is refused: what a scope grants is the audience's to
/// judge, and the issuer carries it to the audience.
pub fn scope(asked: Option<&str>) -> String {
    let mut values: Vec<&str> = Vec::new();
    for value in asked.unwrap_or_default().split_ascii_whitespace() {
        if !values.contains(&value) {
            values.push(value);
        }
    }
    values.join(" ")
}

/// Whether the scope `scope` holds `value`.
pub fn holds(scope: &str, value: &str) -> bool {
    scope.split(' ').any(|held| held == value)
}

/// The audience a request's access token is for, as RFC 8707's
/// `resource` names it: the resource asked for, which must be one of
/// the client's audiences, else `invalid_target`; or, where none is
/// asked for, the client's first.
pub fn audience(client: &Client, resource: Option<&str>) -> Result<String, OAuthError> {
    match resource {
        Some(resource) if client.audiences.iter().any(|audience| audience == resource) => Ok(resource.to_string()),
        Some(resource) => Err(OAuthError::bad("invalid_target", format!("`{resource}` is not an audience of the client"))),
        None => client.audiences.first().cloned().ok_or_else(|| OAuthError::bad("invalid_target", "the client has no audience")),
    }
}
