//! `GET` and `POST /device`: where a person enters a user code.

use std::sync::Arc;

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use super::{DEVICE, escape, page, paragraph, sign_in, user_code};
use crate::config::Config;
use crate::password;
use crate::serve::Issuer;

/// The page's query: the user code, where the device's link carried
/// one.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeviceQuery {
    /// The user code.
    pub user_code: Option<String>,
}

/// The page's form.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeviceDecision {
    /// The user code, as typed.
    pub user_code: Option<String>,
    /// The identity signing in.
    pub subject: Option<String>,
    /// Its password.
    pub password: Option<String>,
    /// `approve` or `deny`.
    pub action: Option<String>,
}

/// `GET /device`: the form — the user code, filled in where the query
/// names one, and the sign-in.
pub async fn device_page(State(issuer): State<Arc<Issuer>>, Query(query): Query<DeviceQuery>) -> Response {
    form(&issuer.config, StatusCode::OK, query.user_code.as_deref(), None)
}

/// `POST /device`: the decision. The rules:
///
/// 1. The user code is read as a person types one — see
///    `user_code::read` — and must be a pending code that has
///    not expired; else the form again, saying so.
/// 2. `deny` denies the code, and needs no sign-in: a person who
///    holds the code may refuse it.
/// 3. `approve` needs the identity's password; a wrong one, or an
///    identity there is no such one of, is the form again, saying only
///    that the two did not match. Right, the code is approved for the
///    identity, signed in now.
pub async fn device_decide(State(issuer): State<Arc<Issuer>>, Form(decision): Form<DeviceDecision>) -> Response {
    let typed = decision.user_code.as_deref().unwrap_or_default();
    let Some(code) = user_code::read(typed) else {
        return form(&issuer.config, StatusCode::BAD_REQUEST, Some(typed), Some("That is not a code."));
    };
    let pending = match issuer.store.devices().pending(&code).await {
        Ok(pending) => pending,
        Err(error) => return failed(&error),
    };
    if pending.is_none() {
        return form(&issuer.config, StatusCode::BAD_REQUEST, Some(typed), Some("That code is not waiting, or it has expired."));
    }
    let subject = match decision.action.as_deref() {
        Some("deny") => None,
        Some("approve") => {
            let subject = decision.subject.unwrap_or_default();
            let hash = match issuer.store.identities().password_hash(&subject).await {
                Ok(hash) => hash,
                Err(error) => return failed(&error),
            };
            if !password::verify(decision.password.as_deref().unwrap_or_default(), hash).await {
                return form(&issuer.config, StatusCode::UNAUTHORIZED, Some(typed), Some("The identity and the password do not match."));
            }
            Some(subject)
        }
        _ => return form(&issuer.config, StatusCode::BAD_REQUEST, Some(typed), Some("Approve or deny the code.")),
    };
    match issuer.store.devices().decide(&code, subject.as_deref()).await {
        Ok(true) if subject.is_some() => page(StatusCode::OK, "Approved", &paragraph("You may return to your device.")),
        Ok(true) => page(StatusCode::OK, "Denied", &paragraph("The device was refused.")),
        Ok(false) => form(&issuer.config, StatusCode::BAD_REQUEST, Some(typed), Some("That code is not waiting, or it has expired.")),
        Err(error) => failed(&error),
    }
}

/// The device page's form, the code filled in, with a note above it.
/// The form posts to the device page's own URL under the issuer, which
/// is where a proxy in front of the issuer serves it.
fn form(config: &Config, status: StatusCode, code: Option<&str>, note: Option<&str>) -> Response {
    let body = format!(
        "{}<form method=\"post\" action=\"{}\">\
         <label>Code<input name=\"user_code\" value=\"{}\" autocomplete=\"off\" required></label>{}</form>",
        note.map(paragraph).unwrap_or_else(|| paragraph("Enter the code your device shows, and sign in.")),
        escape(&config.endpoint(DEVICE.trim_start_matches('/'))),
        escape(code.unwrap_or_default()),
        sign_in("Approve"),
    );
    page(status, "Sign in a device", &body)
}

/// The page a failure of the issuer's own is.
fn failed(error: &crate::store::Error) -> Response {
    page(StatusCode::INTERNAL_SERVER_ERROR, "Something failed", &paragraph(&error.to_string()))
}
