//! `POST /register`: the arguments and the template, once.

use axum::Json;
use axum::http::StatusCode;
use diverge_sdk::container_proxy::inside::register;
use diverge_sdk::container_proxy::inside::register::response::Response;
use serde_json::{Value, json};
use tokio::sync::OnceCell;

use crate::arguments::Arguments;
use crate::permissions::permissions;

/// What registration fixed for the container's life.
#[derive(Debug, Clone)]
pub struct Registration {
    /// The tool template the container was made from, by id. Every
    /// row the courier keeps carries it, so couriers made from
    /// different templates never share mail.
    pub template: String,
}

/// The one registration.
static REGISTRATION: OnceCell<Registration> = OnceCell::const_new();

/// The registration, once it has happened.
pub fn registered() -> Option<&'static Registration> {
    REGISTRATION.get()
}

/// `POST /register`: arguments that are not [`Arguments`] are `400`;
/// a second registration, whatever it carries, is `409`; otherwise
/// `200` with the courier's [`permissions`] — it depends on nothing and
/// takes no secret.
pub async fn register(Json(request): Json<register::request::Request>) -> Result<(StatusCode, Json<Response>), (StatusCode, Json<Value>)> {
    if let Err(error) = serde_json::from_value::<Arguments>(request.arguments) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "kind": "arguments", "error": error.to_string() }))));
    }
    match REGISTRATION.set(Registration { template: request.template }) {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(Response {
                permissions: permissions(),
                ..Response::default()
            }),
        )),
        Err(_) => Err((StatusCode::CONFLICT, Json(json!({ "kind": "registered", "error": "the courier is registered, and it never changes" })))),
    }
}
