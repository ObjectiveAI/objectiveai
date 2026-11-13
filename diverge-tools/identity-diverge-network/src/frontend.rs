//! The frontend, beneath `/frontend`: the page, and what it asks.
//!
//! The page is where a person signs a GitHub or Google account in to
//! the tool, sees the ones it holds, and removes one. The rules:
//!
//! 1. `GET /frontend` sends the page on to `/frontend/`, where it is
//!    served whole: one HTML file, its script and its style inline.
//! 2. `GET /frontend/api/issuers` is the issuers an account may be
//!    signed in at, by name: the SDK's table.
//! 3. `GET /frontend/api/accounts` is every account the tool holds,
//!    each with the name its issuer has for it now — GitHub's `login`,
//!    Google's `email` — asked of the issuer as the page loads and
//!    kept nowhere. An account whose issuer no longer answers for it
//!    is listed all the same, with why in place of its name.
//! 4. `POST /frontend/api/logins` `{"issuer"}` begins a login at that
//!    issuer, by a device code, and answers what the person needs —
//!    the code, and the issuer's page — and the id to ask after it
//!    by. The tool waits on it apart from the request, and keeps the
//!    account an approval is — replacing one it holds already.
//! 5. `GET /frontend/api/logins/{id}` is how that login stands:
//!    `waiting`, `signed_in` with the account, or `failed` with why.
//! 6. `DELETE /frontend/api/accounts/{issuer}/{id}` forgets an
//!    account: `204`, or `404` where the tool holds none such.
//! 7. A request with no stamp, or one stamped as an agent, is `403` —
//!    see [`person`]. A tool not yet registered is `503`.
//! 8. No request answers a token, and none is in the page.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use diverge_sdk::identity_diverge_network::{self, ISSUERS, Which};
use futures_util::future::join_all;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::logins::{Login, Standing};
use crate::person::person;
use crate::register;
use crate::server::{Server, State as Shared};
use crate::store;

/// The page, whole.
const PAGE: &str = include_str!("page.html");

/// The routes beneath `/frontend`, holding the server.
pub fn routes(server: Server) -> Router {
    Router::new()
        .route("/frontend", get(|| async { Redirect::permanent("/frontend/") }))
        .route("/frontend/", get(page))
        .route("/frontend/api/issuers", get(issuers))
        .route("/frontend/api/accounts", get(accounts))
        .route("/frontend/api/accounts/{issuer}/{id}", delete(remove))
        .route("/frontend/api/logins", post(begin))
        .route("/frontend/api/logins/{id}", get(login))
        .with_state(server)
}

/// Now, in seconds since the Unix epoch.
fn now() -> i64 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

/// A failure, as the page reads it.
fn refused(status: StatusCode, failure: impl std::fmt::Display) -> Response {
    (status, Json(json!({ "error": failure.to_string() }))).into_response()
}

/// Rule 7: the person, and the template the accounts are kept under.
fn asked(headers: &HeaderMap) -> Result<&'static str, Response> {
    person(headers).map_err(|failure| refused(StatusCode::FORBIDDEN, failure))?;
    register::registered().map(|registration| registration.template.as_str()).ok_or_else(|| refused(StatusCode::SERVICE_UNAVAILABLE, Failure::Unregistered))
}

/// Rule 1.
async fn page(headers: HeaderMap) -> Response {
    match person(&headers) {
        Ok(()) => Html(PAGE).into_response(),
        Err(failure) => refused(StatusCode::FORBIDDEN, failure),
    }
}

/// Rule 2.
async fn issuers(headers: HeaderMap) -> Response {
    if let Err(failure) = person(&headers) {
        return refused(StatusCode::FORBIDDEN, failure);
    }
    Json(json!({ "issuers": ISSUERS.map(|issuer| issuer.name) })).into_response()
}

/// Rule 3.
async fn accounts(State(server): State<Server>, headers: HeaderMap) -> Response {
    let template = match asked(&headers) {
        Ok(template) => template,
        Err(response) => return response,
    };
    let state = &server.state;
    let listed = match state.store.pool(&state.client).await {
        Ok(pool) => store::accounts::list(pool, template).await,
        Err(failure) => Err(failure),
    };
    let listed = match listed {
        Ok(listed) => listed,
        Err(failure) => return refused(StatusCode::BAD_GATEWAY, failure),
    };
    // Each account's name is its issuer's to say, asked now, all at
    // once: one that cannot be had is said in its place.
    let named = join_all(listed.iter().map(|account| name(state, template, &account.issuer, &account.id))).await;
    let accounts: Vec<Value> = listed
        .into_iter()
        .zip(named)
        .map(|(account, named)| match named {
            Ok(name) => json!({ "account": account.account, "issuer": account.issuer, "id": account.id, "name": name }),
            Err(failure) => json!({ "account": account.account, "issuer": account.issuer, "id": account.id, "name": null, "problem": failure.to_string() }),
        })
        .collect();
    Json(json!({ "accounts": accounts })).into_response()
}

/// The name an account's issuer has for it now: the first of `login`,
/// `email` and `name` its details hold — GitHub's is `login`, Google's
/// `email` — or none where they hold none of them.
async fn name(state: &Shared, template: &str, issuer: &str, id: &str) -> Result<Option<String>, Failure> {
    let issuer = identity_diverge_network::issuer(issuer).map_err(|error| Failure::Issuer(error.to_string()))?;
    let pool = state.store.pool(&state.client).await?;
    let token = store::accounts::token(pool, &state.http, template, issuer, id, Which::Access).await?;
    let info = identity_diverge_network::info(&state.http, issuer, &token).await.map_err(|error| match error {
        identity_diverge_network::Error::Refused { .. } => Failure::SignedOut(format!("{}.{id}", issuer.name)),
        error => Failure::Identity(error.to_string()),
    })?;
    Ok(["login", "email", "name"].iter().find_map(|member| info.get(member).and_then(Value::as_str)).map(str::to_string))
}

/// What a login is begun with.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Begin {
    /// The issuer, by name.
    issuer: String,
}

/// Rule 4.
async fn begin(State(server): State<Server>, headers: HeaderMap, Json(begin): Json<Begin>) -> Response {
    let template = match asked(&headers) {
        Ok(template) => template,
        Err(response) => return response,
    };
    let issuer = match identity_diverge_network::issuer(&begin.issuer) {
        Ok(issuer) => issuer,
        Err(error) => return refused(StatusCode::BAD_REQUEST, error),
    };
    let device = match identity_diverge_network::begin(&server.state.http, issuer).await {
        Ok(device) => device,
        Err(error) => return refused(StatusCode::BAD_GATEWAY, error),
    };
    let now = now();
    let held = Login {
        issuer: issuer.name,
        user_code: device.user_code.clone(),
        verification_uri: device.verification_uri.clone(),
        verification_uri_complete: device.verification_uri_complete.clone(),
        expires: now.saturating_add(i64::try_from(device.expires_in).unwrap_or(i64::MAX)),
        standing: Standing::Waiting,
    };
    let id = server.state.logins.begin(held.clone(), now).await;
    // The wait, apart from the request: the page asks after it.
    let waiting = server.clone();
    tokio::spawn(async move {
        let state = &waiting.state;
        let standing = match device.wait(&state.http).await {
            Ok((user, tokens)) => {
                let kept = match state.store.pool(&state.client).await {
                    Ok(pool) => store::accounts::add(pool, template, issuer.name, &user, &tokens).await,
                    Err(failure) => Err(failure),
                };
                match kept {
                    Ok(()) => Standing::SignedIn { account: format!("{}.{user}", issuer.name) },
                    Err(failure) => Standing::Failed { error: failure.to_string() },
                }
            }
            Err(error) => Standing::Failed { error: error.to_string() },
        };
        state.logins.settle(id, standing).await;
    });
    Json(json!({ "id": id, "login": held })).into_response()
}

/// Rule 5.
async fn login(State(server): State<Server>, headers: HeaderMap, Path(id): Path<u64>) -> Response {
    if let Err(failure) = person(&headers) {
        return refused(StatusCode::FORBIDDEN, failure);
    }
    match server.state.logins.get(id).await {
        Some(login) => Json(json!({ "id": id, "login": login })).into_response(),
        None => refused(StatusCode::NOT_FOUND, "there is no such login: it ended long ago, or the tool restarted"),
    }
}

/// Rule 6.
async fn remove(State(server): State<Server>, headers: HeaderMap, Path((issuer, id)): Path<(String, String)>) -> Response {
    let template = match asked(&headers) {
        Ok(template) => template,
        Err(response) => return response,
    };
    let state = &server.state;
    let removed = match state.store.pool(&state.client).await {
        Ok(pool) => store::accounts::remove(pool, template, &issuer, &id).await,
        Err(failure) => Err(failure),
    };
    match removed {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => refused(StatusCode::NOT_FOUND, Failure::NoAccount(format!("{issuer}.{id}"))),
        Err(failure) => refused(StatusCode::BAD_GATEWAY, failure),
    }
}
