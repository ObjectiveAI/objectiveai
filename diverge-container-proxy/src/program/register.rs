//! The arguments and the template, told to the program's server
//! once: `POST /register`, answered with the dependencies the program
//! declares, the secrets it takes and the permissions it holds.

use diverge_sdk::container_proxy::inside::register::request::Request;
use diverge_sdk::container_proxy::inside::register::response::Response;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::permission::Grant;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use super::{Upstream, refused, status_error};

/// What the program's server answered the registration with.
pub struct Registered {
    /// The dependencies the program declares, which an agent
    /// container's `Begun` carries and a tool container's begin
    /// drops.
    pub dependencies: Vec<Template>,
    /// The secrets the program takes, by name, each `fixed` or
    /// `rotating`: what the proxy feeds the program at every
    /// activation, and — the rotating ones — collects from it at
    /// every inactive. Empty is a program that takes none, whose
    /// gate is never closed.
    pub secrets: Declaration,
    /// The grants the program holds over the daemon, which a tool
    /// container's `Begun` carries and an agent container's begin
    /// drops.
    pub permissions: Vec<Grant>,
}

/// Register `arguments` and `template`, the id of the template the
/// container is made from, with the program's server. A `2xx` is the
/// two held for the container's life, and its body the dependencies
/// the program declares, the secrets it takes and the permissions it
/// holds; a non-`2xx` is the
/// image refusing them, in its own words, and a server that cannot be
/// reached is the refusal too. So is a `2xx` whose body is not the
/// response: the dependencies, the declaration and the permissions are
/// part of
/// registration, and a registration the proxy cannot read has not
/// happened. No value of any secret is here: values arrive on the
/// begin scope's secrets channel, after `Begun`.
pub async fn register(upstream: &Upstream, arguments: Value, template: String) -> Result<Registered, Error> {
    let body = serde_json::to_vec(&Request { arguments, template }).map_err(|error| {
        Error(serde_json::json!({
            "kind": "program",
            "error": format!("the arguments did not serialize: {error}"),
        }))
    })?;
    let response = upstream
        .http()
        .post(upstream.url("/register"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await;
    let response = match response {
        Err(error) => return Err(refused(&error)),
        Ok(response) if !response.status().is_success() => return Err(status_error(response).await),
        Ok(response) => response,
    };
    let body = response.bytes().await.map_err(|error| refused(&error))?;
    serde_json::from_slice::<Response>(&body)
        .map(|response| Registered {
            dependencies: response.dependencies,
            secrets: response.secrets,
            permissions: response.permissions,
        })
        .map_err(|error| {
            Error(serde_json::json!({
                "kind": "program",
                "error": format!("the register answer did not parse: {error}"),
            }))
        })
}
