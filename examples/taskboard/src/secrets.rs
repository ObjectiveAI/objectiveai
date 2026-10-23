//! `POST /secrets` and `POST /secrets/collect`: the tool takes none.

use axum::Json;
use axum::http::StatusCode;
use diverge_sdk::container_proxy::inside::secrets::{collect, feed};
use serde_json::{Value, json};

/// `POST /secrets`: the values fed. The tool declared no secret, so
/// there is nothing to hold; `204` all the same, as the contract asks
/// of every program.
pub async fn feed(Json(_request): Json<feed::request::Request>) -> StatusCode {
    StatusCode::NO_CONTENT
}

/// `POST /secrets/collect`: the rotating secrets asked back. The tool
/// declared none, so the proxy never asks; served as the contract all
/// the same: no names is an empty answer, and any name is `500`,
/// naming it, since the tool holds no value for it.
pub async fn collect(Json(request): Json<collect::request::Request>) -> Result<Json<collect::response::Response>, (StatusCode, Json<Value>)> {
    match request.names.first() {
        None => Ok(Json(collect::response::Response(Default::default()))),
        Some(name) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "kind": "secrets", "error": format!("the taskboard holds no secret named {name}") })))),
    }
}
