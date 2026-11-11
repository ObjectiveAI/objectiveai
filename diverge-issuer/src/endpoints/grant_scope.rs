//! A request's scope and audience.

use super::CLIENT_ID;

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

/// The audience a request's access token is for: the resource asked
/// for, as RFC 8707's `resource` names it, or the issuer's one app,
/// [`CLIENT_ID`], where none is asked for. No resource is refused:
/// which servers admit a token is theirs to judge.
pub fn audience(resource: Option<&str>) -> String {
    resource.unwrap_or(CLIENT_ID).to_string()
}
