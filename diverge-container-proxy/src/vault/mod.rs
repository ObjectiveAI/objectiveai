//! The vault, from the inside: `/vault/<op>`, each one ask.
//!
//! The program beside the proxy has no MCP for the vault, so the
//! proxy gives it plain HTTP: one `POST` per operation, its body the
//! wire's own request payload, its answer the wire's own one message.
//! The proxy decodes a body only to validate it and to type the ask,
//! and hands the answer back untouched — the SDK on the other end
//! uses the same codecs.
//!
//! Beside `200`: `400` for a body that would not decode, `502` for an
//! ask that died or was refused, `500` for one that would not encode.
//! No retry, per the wire: a vault operation is not safe to repeat.
//! A `lock` holds its request open until the lock is held; nothing
//! times out. [`ask`] is the one relay under all five, and the file
//! mounts under [`filesystem`](crate::filesystem) ask through it too.

pub mod ask;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::requests::request::Request;
use diverge_provider_sdk::container_proxy::vault;

use crate::requests::Requests;

/// Ask once, and answer with what came back.
async fn relay(requests: &Requests, request: Request<'_>) -> Response {
    match ask::ask(requests, request).await {
        Ok(bytes) => (StatusCode::OK, bytes).into_response(),
        Err(ask::Asked::Encode) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Err(ask::Asked::Empty) | Err(ask::Asked::Died) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

/// `POST /vault/get`.
pub async fn get(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    match vault::get::request::Request::decode(&body) {
        Ok(request) => relay(&requests, Request::VaultGet(request)).await,
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

/// `POST /vault/set`.
pub async fn set(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    match vault::set::request::Request::decode(&body) {
        Ok(request) => relay(&requests, Request::VaultSet(request)).await,
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

/// `POST /vault/delete`.
pub async fn delete(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    match vault::delete::request::Request::decode(&body) {
        Ok(request) => relay(&requests, Request::VaultDelete(request)).await,
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

/// `POST /vault/lock`.
pub async fn lock(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    match vault::lock::request::Request::decode(&body) {
        Ok(request) => relay(&requests, Request::VaultLock(request)).await,
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

/// `POST /vault/unlock`.
pub async fn unlock(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    match vault::unlock::request::Request::decode(&body) {
        Ok(request) => relay(&requests, Request::VaultUnlock(request)).await,
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}
