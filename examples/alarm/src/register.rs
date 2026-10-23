//! `POST /register`: the arguments, once.

use axum::Json;
use axum::http::StatusCode;
use diverge_sdk::container_proxy::inside::register;
use diverge_sdk::container_proxy::inside::register::response::Response;
use serde_json::{Value, json};
use tokio::sync::OnceCell;

use crate::arguments::Arguments;

/// Whether the one registration has happened.
static REGISTERED: OnceCell<()> = OnceCell::const_new();

/// `POST /register`: arguments that are not [`Arguments`] are `400`;
/// a second registration, whatever it carries, is `409`; otherwise
/// `200` with an empty answer — the tool depends on nothing and takes
/// no secret.
pub async fn register(Json(request): Json<register::request::Request>) -> Result<(StatusCode, Json<Response>), (StatusCode, Json<Value>)> {
    if let Err(error) = serde_json::from_value::<Arguments>(request.arguments) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "kind": "arguments", "error": error.to_string() }))));
    }
    match REGISTERED.set(()) {
        Ok(()) => Ok((StatusCode::OK, Json(Response::default()))),
        Err(_) => Err((StatusCode::CONFLICT, Json(json!({ "kind": "registered", "error": "the alarm tool is registered, and it never changes" })))),
    }
}
