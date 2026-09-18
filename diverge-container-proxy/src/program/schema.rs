//! The arguments' schema: `GET /schema`, answered on the server's
//! channel.

use std::sync::Arc;

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::containers::schema::response;
use diverge_provider_sdk::shared::error::Error;
use serde_json::Value;

use super::{refused, status_error};
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Ask the program's server for the JSON Schema of the arguments and
/// answer the channel with it: one frame, then the finish. A
/// non-`2xx`, a server that cannot be reached, or a body that is not
/// JSON is the `Error` — a schema is a courtesy an image extends, not
/// an obligation, and a server that wants one and gets none knows so.
pub async fn schema(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32) {
    let response = proxy.upstream.http().get(proxy.upstream.url("/schema")).send().await;
    let frame = match response {
        Err(error) => response::Frame::Error(refused(&error)),
        Ok(response) if !response.status().is_success() => response::Frame::Error(status_error(response).await),
        Ok(response) => match response.bytes().await {
            Err(error) => response::Frame::Error(refused(&error)),
            Ok(body) => match serde_json::from_slice::<Value>(&body) {
                Ok(schema) => response::Frame::Schema(schema),
                Err(error) => response::Frame::Error(Error(serde_json::json!({
                    "kind": "program",
                    "error": format!("the schema did not parse: {error}"),
                }))),
            },
        },
    };
    reply(&scope, channel, encoded(&frame)).await;
}
