//! `GET`, `POST /authorize`: the authorization endpoint, OpenID Connect
//! Core §3.1.2, signed in by Basic credentials — never a page.

use std::sync::Arc;

use axum::extract::rejection::{FormRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::Form;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;

use super::{CLIENT_ID, OAuthError, audience, scope};
use crate::password;
use crate::serve::Issuer;
use crate::store::codes::Bound;
use crate::tokens;

/// The realm the credentials are asked for in.
const REALM: &str = "Basic realm=\"diverge\", charset=\"UTF-8\"";

/// An authorization request's parameters, as a query or a form.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthorizeRequest {
    /// Must be `code`.
    pub response_type: Option<String>,
    /// Must be the issuer's one app.
    pub client_id: Option<String>,
    /// Where the code is sent: a loopback `http` URI.
    pub redirect_uri: Option<String>,
    /// The scope asked for.
    pub scope: Option<String>,
    /// Sent back with the code, as the client gave it.
    pub state: Option<String>,
    /// Put in the ID token, as the client gave it.
    pub nonce: Option<String>,
    /// The PKCE challenge, RFC 7636.
    pub code_challenge: Option<String>,
    /// Must be `S256`.
    pub code_challenge_method: Option<String>,
    /// The audience asked for, RFC 8707's.
    pub resource: Option<String>,
}

/// `GET /authorize`: the request as its query — see [`authorize`].
pub async fn authorize_get(State(issuer): State<Arc<Issuer>>, headers: HeaderMap, request: Result<Query<AuthorizeRequest>, QueryRejection>) -> Response {
    match request {
        Ok(Query(request)) => authorize(&issuer, &headers, request).await,
        Err(rejection) => OAuthError::invalid_request(rejection.body_text()).into_response(),
    }
}

/// `POST /authorize`: the request as its form — see [`authorize`].
pub async fn authorize_post(State(issuer): State<Arc<Issuer>>, headers: HeaderMap, request: Result<Form<AuthorizeRequest>, FormRejection>) -> Response {
    match request {
        Ok(Form(request)) => authorize(&issuer, &headers, request).await,
        Err(rejection) => OAuthError::invalid_request(rejection.body_text()).into_response(),
    }
}

/// The authorization request, answered. The rules:
///
/// 1. **The request**, checked before anything else: `response_type`
///    `code`, `client_id` the issuer's one app, `redirect_uri` an `http`
///    URI on `127.0.0.1` or `[::1]` at any port — RFC 8252 §7.3's
///    loopback redirect — and a `code_challenge` with
///    `code_challenge_method` `S256`. A request that fails is `400` with
///    the OAuth error, never a redirect: its redirect URI is not yet
///    trusted.
/// 2. **The credentials.** The subject and its password come as Basic
///    credentials, RFC 7617, and are checked as every password is — in
///    the same time whether or not there is such an identity. None, or
///    wrong, is `401` with `WWW-Authenticate: Basic` and a plain-text
///    body: an agent sends the header itself, and a browser asks for it
///    in its own prompt. Nothing the issuer answers is a page.
/// 3. **The code.** Right, a fresh code is issued, bound as
///    [`codes`](crate::store::codes) states, and the answer is `302`
///    to the redirect URI with the code and the `state`, RFC 6749
///    §4.1.2.
pub async fn authorize(issuer: &Issuer, headers: &HeaderMap, request: AuthorizeRequest) -> Response {
    // Rule 1.
    let checked = (|| {
        if request.response_type.as_deref() != Some("code") {
            return Err(OAuthError::bad("unsupported_response_type", "this issuer answers only `response_type=code`"));
        }
        if request.client_id.as_deref() != Some(CLIENT_ID) {
            return Err(OAuthError::invalid_client(false));
        }
        let redirect_uri = request.redirect_uri.as_deref().ok_or_else(|| OAuthError::invalid_request("`redirect_uri` is required"))?;
        let url = url::Url::parse(redirect_uri).map_err(|_| OAuthError::invalid_request("`redirect_uri` is no URL"))?;
        let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
        if url.scheme() != "http" || !loopback || url.fragment().is_some() {
            return Err(OAuthError::invalid_request("`redirect_uri` must be `http` on `127.0.0.1` or `[::1]`, RFC 8252's loopback redirect"));
        }
        let challenge = request.code_challenge.as_deref().ok_or_else(|| OAuthError::invalid_request("`code_challenge` is required"))?;
        if request.code_challenge_method.as_deref() != Some("S256") {
            return Err(OAuthError::invalid_request("`code_challenge_method` must be `S256`"));
        }
        Ok((url, challenge.to_string()))
    })();
    let (mut url, challenge) = match checked {
        Ok(checked) => checked,
        Err(error) => return error.into_response(),
    };
    // Rule 2.
    let Some((subject, given)) = credentials(headers) else {
        return unauthorized();
    };
    let hash = match issuer.store.identities().password_hash(&subject).await {
        Ok(hash) => hash,
        Err(error) => return OAuthError::from(error).into_response(),
    };
    if !password::verify(&given, hash).await {
        return unauthorized();
    }
    // Rule 3.
    let code = tokens::mint();
    let bound = Bound {
        subject,
        redirect_uri: request.redirect_uri.clone().unwrap_or_default(),
        challenge,
        nonce: request.nonce.clone(),
        scope: scope(request.scope.as_deref()),
        audience: audience(request.resource.as_deref()),
        auth_time: crate::now(),
    };
    if let Err(error) = issuer.store.codes().issue(&tokens::hash(&code), &bound).await {
        return OAuthError::from(error).into_response();
    }
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("code", &code);
        if let Some(state) = &request.state {
            query.append_pair("state", state);
        }
    }
    let Ok(location) = HeaderValue::from_str(url.as_str()) else {
        return OAuthError::server("the redirect is no header value").into_response();
    };
    (StatusCode::FOUND, [(header::LOCATION, location), (header::CACHE_CONTROL, HeaderValue::from_static("no-store"))]).into_response()
}

/// The subject and the password of the request's Basic credentials, if
/// it carries them.
fn credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ").or_else(|| value.strip_prefix("basic "))?;
    let decoded = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let (subject, password) = decoded.split_once(':')?;
    Some((subject.to_string(), password.to_string()))
}

/// Rule 2's refusal: `401`, the realm, and why, in plain text.
fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, REALM), (header::CONTENT_TYPE, "text/plain; charset=utf-8"), (header::CACHE_CONTROL, "no-store")],
        "Basic credentials are required: the subject and its password.\n",
    )
        .into_response()
}
