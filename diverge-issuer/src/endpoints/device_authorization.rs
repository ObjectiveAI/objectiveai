//! `POST /device_authorization`: RFC 8628 §3.1.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::FormRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};

use super::{DEVICE, OAuthError, audience, authenticate, no_store, scope, user_code};
use crate::serve::Issuer;
use crate::store::clients::Grant;
use crate::store::devices::New;
use crate::tokens;

/// How many user codes are drawn before a request gives up: a draw
/// is refused only when it is another pending code's, which with
/// eight characters of twenty is next to never.
const DRAWS: usize = 8;

/// The request's form.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeviceForm {
    /// The client's id, where it does not authenticate with `Basic`.
    pub client_id: Option<String>,
    /// The client's secret, where it does not authenticate with
    /// `Basic`.
    pub client_secret: Option<String>,
    /// The scope asked for.
    pub scope: Option<String>,
    /// The audience asked for, RFC 8707's.
    pub resource: Option<String>,
}

/// The answer, RFC 8628 §3.2.
#[derive(Debug, Clone, Serialize)]
struct Body {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: u64,
    interval: u64,
}

/// `POST /device_authorization`: a device code for the client, and the
/// user code a person types on the device page. The client is
/// authenticated as at the token endpoint and must have
/// `device_code`; the audience is resolved now, as the token request
/// will not name one. The device code is kept as its SHA-256.
pub async fn device_authorization(
    State(issuer): State<Arc<Issuer>>,
    headers: HeaderMap,
    form: Result<Form<DeviceForm>, FormRejection>,
) -> Response {
    match authorize(&issuer, &headers, form).await {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

/// The request, answered.
async fn authorize(issuer: &Issuer, headers: &HeaderMap, form: Result<Form<DeviceForm>, FormRejection>) -> Result<Response, OAuthError> {
    let Form(form) = form.map_err(|rejection| OAuthError::invalid_request(rejection.body_text()))?;
    let client = authenticate(issuer, headers, form.client_id.as_deref(), form.client_secret.as_deref()).await?;
    if !client.may(Grant::DeviceCode) {
        return Err(OAuthError::unauthorized_client());
    }
    let config = &issuer.config;
    let audience = audience(&client, form.resource.as_deref())?;
    let scope = scope(form.scope.as_deref());
    let device_code = tokens::mint();
    let expires = crate::now().saturating_add(i64::try_from(config.device_code_seconds).unwrap_or(i64::MAX));
    for _ in 0..DRAWS {
        let code = user_code::draw();
        let device = New {
            hash: tokens::hash(device_code.expose()),
            user_code: code.clone(),
            client: client.id.clone(),
            scope: scope.clone(),
            audience: audience.clone(),
            expires,
            interval: i64::try_from(config.device_interval_seconds).unwrap_or(i64::MAX),
        };
        if issuer.store.devices().add(&device).await? {
            let shown = user_code::show(&code);
            let verification_uri = config.endpoint(DEVICE.trim_start_matches('/'));
            let mut complete = url::Url::parse(&verification_uri).map_err(OAuthError::server)?;
            complete.query_pairs_mut().append_pair("user_code", &shown);
            let body = Body {
                device_code: device_code.expose().to_string(),
                user_code: shown,
                verification_uri,
                verification_uri_complete: complete.to_string(),
                expires_in: config.device_code_seconds,
                interval: config.device_interval_seconds,
            };
            return Ok((no_store(), Json(body)).into_response());
        }
    }
    Err(OAuthError::server("no user code could be drawn that is not held"))
}
