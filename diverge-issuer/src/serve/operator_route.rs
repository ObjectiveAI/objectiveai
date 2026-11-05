//! The operator's word on device codes: `POST /pending`, `POST
//! /approve` and `POST /deny`, from loopback only.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::connect_info::ConnectInfo;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use serde::Deserialize;
use serde_json::json;

use super::Issuer;
use crate::endpoints::user_code;

/// What `/approve` and `/deny` are asked.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Decision {
    /// The user code, as a person types it or as it is shown.
    pub user_code: Option<String>,
}

/// The rules, for all three:
///
/// 1. **Loopback only.** Only a loopback peer is heard, as at `/stop`
///    and `/rotate`: any other is answered `403 Forbidden` with nothing
///    done. Loopback is the whole credential — the operator is whoever
///    is on the issuer's host.
/// 2. **Waiting.** `/pending` answers `200 OK` with every pending code
///    that has not expired, the soonest to expire first, as
///    `[{"user_code","subject","expires"}]`: the user code as it is
///    shown, the subject the login asked for.
/// 3. **Approving.** `/approve` with the form `user_code` approves that
///    code as the subject it asked for — the subject made an identity
///    if it was not one — and answers `200 OK` with
///    `{"approved":{"user_code","subject"}}`.
/// 4. **Denying.** `/deny` denies it, and answers `200 OK` with
///    `{"denied":{"user_code"}}`.
/// 5. **No such code.** A user code that is none, or no pending code
///    that has not expired, is `404 Not Found` with `{"error"}`, and
///    nothing is decided.
/// 6. **A failure** of the database is `500` with `{"error"}`.
pub async fn pending(State(issuer): State<Arc<Issuer>>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> Response {
    // Rule 1.
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    // Rule 2.
    match issuer.store.devices().waiting().await {
        Ok(waiting) => {
            let body: Vec<_> = waiting
                .into_iter()
                .map(|code| json!({ "user_code": user_code::show(&code.user_code), "subject": code.subject, "expires": code.expires }))
                .collect();
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(error) => failed(error),
    }
}

/// `POST /approve`: see [`pending`] for the rules.
pub async fn approve(State(issuer): State<Arc<Issuer>>, ConnectInfo(peer): ConnectInfo<SocketAddr>, Form(decision): Form<Decision>) -> Response {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(code) = decision.user_code.as_deref().and_then(user_code::read) else {
        return none();
    };
    // Rules 3 and 5.
    match issuer.store.devices().approve(&code).await {
        Ok(Some(subject)) => (StatusCode::OK, Json(json!({ "approved": { "user_code": user_code::show(&code), "subject": subject } }))).into_response(),
        Ok(None) => none(),
        Err(error) => failed(error),
    }
}

/// `POST /deny`: see [`pending`] for the rules.
pub async fn deny(State(issuer): State<Arc<Issuer>>, ConnectInfo(peer): ConnectInfo<SocketAddr>, Form(decision): Form<Decision>) -> Response {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(code) = decision.user_code.as_deref().and_then(user_code::read) else {
        return none();
    };
    // Rules 4 and 5.
    match issuer.store.devices().deny(&code).await {
        Ok(true) => (StatusCode::OK, Json(json!({ "denied": { "user_code": user_code::show(&code) } }))).into_response(),
        Ok(false) => none(),
        Err(error) => failed(error),
    }
}

/// Rule 5.
fn none() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "no pending code is that one" }))).into_response()
}

/// Rule 6.
fn failed(error: impl std::fmt::Display) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": error.to_string() }))).into_response()
}
