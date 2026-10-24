//! `GET` and `POST /authorize`: the authorization code, with PKCE.

use std::sync::Arc;

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use url::Url;

use super::{AUTHORIZE, audience, escape, hidden, page, paragraph, pkce, scope, sign_in};
use crate::password;
use crate::serve::Issuer;
use crate::store::clients::Grant;
use crate::store::codes::Code;
use crate::tokens;

/// An authorization request's parameters, OpenID Connect Core §3.1.2.1
/// and RFC 7636 §4.3, as the page is opened with them and as its form
/// posts them back; the sign-in's fields beside them on the post.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthorizeRequest {
    /// `code`, the one response type.
    pub response_type: Option<String>,
    /// The client.
    pub client_id: Option<String>,
    /// Where the code is sent.
    pub redirect_uri: Option<String>,
    /// The scope asked for.
    pub scope: Option<String>,
    /// The client's own state, sent back as it was.
    pub state: Option<String>,
    /// The client's nonce, for the ID token.
    pub nonce: Option<String>,
    /// The PKCE challenge.
    pub code_challenge: Option<String>,
    /// `S256`, the one PKCE method.
    pub code_challenge_method: Option<String>,
    /// The audience asked for, RFC 8707's.
    pub resource: Option<String>,
    /// OpenID Connect's `prompt`.
    pub prompt: Option<String>,
    /// The identity signing in, on the post.
    pub subject: Option<String>,
    /// Its password, on the post.
    pub password: Option<String>,
    /// `approve` or `deny`, on the post.
    pub action: Option<String>,
}

/// A request that passed every check: what a code is issued for.
struct Checked {
    client: String,
    redirect_uri: String,
    scope: String,
    audience: String,
    challenge: String,
}

/// `GET /authorize`: the sign-in page, once the request passes
/// its checks; the request's parameters ride its form as hidden
/// fields.
pub async fn authorize_page(State(issuer): State<Arc<Issuer>>, Query(request): Query<AuthorizeRequest>) -> Response {
    match check(&issuer, &request).await {
        Ok(checked) => sign_in_page(&issuer, &request, &checked, StatusCode::OK, None),
        Err(refusal) => refusal,
    }
}

/// `POST /authorize`: the sign-in. The request is checked again, as
/// the page's form is the person's browser's to change. Then:
///
/// 1. `deny` sends the client `access_denied`, with no sign-in.
/// 2. `approve` needs the identity's password; a wrong one, or an
///    identity there is no such one of, is the page again, saying
///    only that the two did not match.
/// 3. Right, a code is issued for the identity, signed in now, good
///    for `code_seconds`, kept as its SHA-256; and the browser is sent
///    to the redirect URI with `code`, `state` and `iss`, RFC 9207's.
pub async fn authorize_decide(State(issuer): State<Arc<Issuer>>, Form(request): Form<AuthorizeRequest>) -> Response {
    let checked = match check(&issuer, &request).await {
        Ok(checked) => checked,
        Err(refusal) => return refusal,
    };
    match request.action.as_deref() {
        Some("approve") => {}
        Some("deny") => return back(&issuer, &checked.redirect_uri, &request, &[("error", "access_denied")]),
        _ => return sign_in_page(&issuer, &request, &checked, StatusCode::BAD_REQUEST, Some("Sign in, or deny.")),
    }
    let subject = request.subject.clone().unwrap_or_default();
    let hash = match issuer.store.identities().password_hash(&subject).await {
        Ok(hash) => hash,
        Err(error) => return failed(&error.to_string()),
    };
    if !password::verify(request.password.as_deref().unwrap_or_default(), hash).await {
        let note = Some("The identity and the password do not match.");
        return sign_in_page(&issuer, &request, &checked, StatusCode::UNAUTHORIZED, note);
    }
    let code = tokens::mint();
    let now = crate::now();
    let kept = Code {
        hash: tokens::hash(code.expose()),
        client: checked.client,
        subject,
        redirect_uri: checked.redirect_uri.clone(),
        challenge: checked.challenge,
        scope: checked.scope,
        audience: checked.audience,
        nonce: request.nonce.clone(),
        auth_time: now,
        expires: now.saturating_add(i64::try_from(issuer.config.code_seconds).unwrap_or(i64::MAX)),
    };
    if let Err(error) = issuer.store.codes().add(&kept).await {
        return failed(&error.to_string());
    }
    back(&issuer, &checked.redirect_uri, &request, &[("code", code.expose())])
}

/// Whether the request may be signed in for. The rules, OpenID Connect
/// Core §3.1.2.2 and §3.1.2.6, in order:
///
/// 1. `client_id` names a client with `authorization_code`, and
///    `redirect_uri` is one of its redirect URIs, exactly. Else the
///    browser is NOT sent back — the redirect URI is not one the
///    issuer may trust — and the answer is a page saying why.
/// 2. Every other refusal sends the browser back to the redirect URI
///    with `error`, `error_description`, `state` and `iss`:
///    `unsupported_response_type` unless `response_type` is `code`;
///    `invalid_request` unless `code_challenge` is an `S256`
///    challenge and `code_challenge_method` is `S256`, since PKCE is
///    required of every client; `invalid_target` for a resource that
///    is not one of the client's audiences; and `login_required` for
///    `prompt=none`, since the issuer keeps no sign-in to answer with.
async fn check(issuer: &Issuer, request: &AuthorizeRequest) -> Result<Checked, Response> {
    let client = match request.client_id.as_deref() {
        Some(id) => match issuer.store.clients().load(id).await {
            Ok(client) => client,
            Err(error) => return Err(failed(&error.to_string())),
        },
        None => None,
    };
    let Some(client) = client.filter(|client| client.may(Grant::AuthorizationCode)) else {
        return Err(refused("The client is not one that may sign people in here."));
    };
    let Some(redirect_uri) = request.redirect_uri.clone().filter(|uri| client.redirects.contains(uri)) else {
        return Err(refused("The redirect URI is not one the client registered."));
    };
    let refuse = |code: &str, description: &str| back(issuer, &redirect_uri, request, &[("error", code), ("error_description", description)]);
    if request.response_type.as_deref() != Some("code") {
        return Err(refuse("unsupported_response_type", "the one response type is `code`"));
    }
    let challenge = request.code_challenge.clone().filter(|challenge| pkce::challenge(challenge));
    let Some(challenge) = challenge.filter(|_| request.code_challenge_method.as_deref() == Some(pkce::S256)) else {
        return Err(refuse("invalid_request", "PKCE is required, with `S256`"));
    };
    let audience = match audience(&client, request.resource.as_deref()) {
        Ok(audience) => audience,
        Err(error) => return Err(refuse(error.code, &error.description)),
    };
    if request.prompt.as_deref().is_some_and(|prompt| prompt.split(' ').any(|value| value == "none")) {
        return Err(refuse("login_required", "the issuer keeps no sign-in; every authorization signs in"));
    }
    Ok(Checked { client: client.id, redirect_uri, scope: scope(request.scope.as_deref()), audience, challenge })
}

/// The browser sent to `redirect_uri` with `pairs`, the request's
/// `state` where it had one, and `iss`, the issuer, added to its
/// query.
fn back(issuer: &Issuer, redirect_uri: &str, request: &AuthorizeRequest, pairs: &[(&str, &str)]) -> Response {
    let Ok(mut url) = Url::parse(redirect_uri) else {
        return refused("The redirect URI is not a URL.");
    };
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            query.append_pair(name, value);
        }
        if let Some(state) = &request.state {
            query.append_pair("state", state);
        }
        query.append_pair("iss", &issuer.config.issuer());
    }
    Redirect::to(url.as_str()).into_response()
}

/// The sign-in page for `request`, with a note above the form where
/// there is one.
fn sign_in_page(issuer: &Issuer, request: &AuthorizeRequest, checked: &Checked, status: StatusCode, note: Option<&str>) -> Response {
    let carried = [
        ("response_type", &request.response_type),
        ("client_id", &request.client_id),
        ("redirect_uri", &request.redirect_uri),
        ("scope", &request.scope),
        ("state", &request.state),
        ("nonce", &request.nonce),
        ("code_challenge", &request.code_challenge),
        ("code_challenge_method", &request.code_challenge_method),
        ("resource", &request.resource),
    ];
    let fields: String = carried.iter().map(|(name, value)| hidden(name, value.as_deref())).collect();
    let asks = format!("{} asks to act as you", checked.client);
    let body = format!(
        "{}{}<form method=\"post\" action=\"{}\">{fields}{}</form>",
        note.map(paragraph).unwrap_or_default(),
        paragraph(&asks),
        escape(&issuer.config.endpoint(AUTHORIZE.trim_start_matches('/'))),
        sign_in("Sign in"),
    );
    page(status, "Sign in", &body)
}

/// The page a request whose client or redirect URI is not to be
/// trusted is answered with.
fn refused(why: &str) -> Response {
    page(StatusCode::BAD_REQUEST, "This sign-in is refused", &paragraph(why))
}

/// The page a failure of the issuer's own is.
fn failed(why: &str) -> Response {
    page(StatusCode::INTERNAL_SERVER_ERROR, "Something failed", &paragraph(why))
}
