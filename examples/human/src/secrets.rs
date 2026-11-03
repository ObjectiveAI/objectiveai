//! `POST /secrets` and `POST /secrets/collect`: the agent takes none.

use axum::Json;
use axum::http::StatusCode;
use diverge_sdk::container_proxy::inside::secrets::{collect, feed};
use serde_json::{Value, json};

/// `POST /secrets`: the values fed. The agent declared no secret, so
/// there is nothing to hold; `204` all the same, as the contract asks
/// of every program.
pub async fn feed(Json(_request): Json<feed::request::Request>) -> StatusCode {
    StatusCode::NO_CONTENT
}

/// `POST /secrets/collect`: the rotating secrets asked back. The agent
/// declared none, so the proxy never asks; served as the contract all
/// the same: no names is an empty answer, and any name is `500`,
/// naming it, since the agent holds no value for it.
pub async fn collect(Json(request): Json<collect::request::Request>) -> Result<Json<collect::response::Response>, (StatusCode, Json<Value>)> {
    match request.names.first() {
        None => Ok(Json(collect::response::Response(Default::default()))),
        Some(name) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "kind": "secrets", "error": format!("the agent holds no secret named {name}") })))),
    }
}
