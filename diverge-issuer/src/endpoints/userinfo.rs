//! `GET` and `POST /userinfo`: OpenID Connect Core §5.3.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::serve::Issuer;
use crate::tokens;

/// The user info endpoint: `{"sub"}` for the subject of the bearer
/// access token the request carries — the one claim the issuer knows
/// of an identity. A request with no bearer token, or one that does
/// not verify — see [`verify`](tokens::verify) — is
/// `401 Unauthorized`, with `WWW-Authenticate` naming
/// `invalid_token`, as RFC 6750 §3 states.
pub async fn userinfo(State(issuer): State<Arc<Issuer>>, headers: HeaderMap) -> Response {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim());
    let verified = {
        let keys = issuer.keys.read().await;
        token.and_then(|token| tokens::verify(&issuer.config, &keys.signing, token))
    };
    match verified {
        Some(access) => Json(json!({ "sub": access.sub })).into_response(),
        None => {
            let mut response = StatusCode::UNAUTHORIZED.into_response();
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer error=\"invalid_token\""));
            response
        }
    }
}
