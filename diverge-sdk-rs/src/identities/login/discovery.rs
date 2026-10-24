//! An issuer's discovery document.

use serde::Deserialize;

use crate::identities::Error;

/// What a login needs of the document, OpenID Connect Discovery §3 and
/// RFC 8628 §4.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Discovery {
    /// The issuer identifier, which must be the one asked for.
    pub(super) issuer: String,
    /// The token endpoint.
    pub(super) token_endpoint: String,
    /// The device authorization endpoint, where the issuer has one.
    pub(super) device_authorization_endpoint: Option<String>,
}

/// The discovery document of `issuer`, at
/// `<issuer>/.well-known/openid-configuration`. The rules:
///
/// 1. `issuer` is an absolute URL whose scheme is `https`, or `http`
///    with a loopback host — `127.0.0.1`, `localhost` or `[::1]` —
///    else [`Error::Issuer`].
/// 2. The document's `issuer` is `issuer`, exactly, else
///    [`Error::Discovery`]: a document that names another issuer is
///    not this one's.
/// 3. It names a device authorization endpoint, else
///    [`Error::Discovery`].
pub(super) async fn discover(http: &reqwest::Client, issuer: &str) -> Result<(Discovery, String), Error> {
    let url = reqwest::Url::parse(issuer).map_err(|_| Error::Issuer(issuer.to_string()))?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        return Err(Error::Issuer(issuer.to_string()));
    }
    let at = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));
    let response = http.get(&at).send().await.map_err(Error::Http)?.error_for_status().map_err(Error::Http)?;
    let document: Discovery = response.json().await.map_err(Error::Http)?;
    if document.issuer != issuer {
        return Err(Error::Discovery(format!("the document at `{at}` names the issuer `{}`", document.issuer)));
    }
    let device = document
        .device_authorization_endpoint
        .clone()
        .ok_or_else(|| Error::Discovery(format!("the document at `{at}` names no device authorization endpoint")))?;
    Ok((document, device))
}
