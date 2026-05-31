//! Generic unary + streaming helpers for the per-endpoint subcommands.
//!
//! Every leaf under `api/<...>/{post,get,delete}.rs` is one of:
//!
//! - `call_unary::<Req, Resp>(...)` — emit one notification, then return.
//! - `call_streaming::<Req, Chunk>(...)` — emit one notification per chunk.
//!
//! For no-body endpoints, `Req = ()` and `body = None`.

use std::sync::Arc;

use futures::StreamExt;
use objectiveai_sdk::cli::output::{Handle, Notification, Output};

/// Apply the per-endpoint `--agent-id` flag to the SDK HttpClient
/// only when `build_http_client` didn't already populate
/// `http.agent_id` from `OBJECTIVEAI_AGENT_ID`. Matches the user's
/// rule: handle's agent_id (env-derived, mirrored on
/// `http.agent_id`) takes precedence; the flag is the fallback
/// header.
fn apply_agent_id_arg(
    http: &mut objectiveai_sdk::HttpClient,
    agent_id_arg: Option<String>,
) {
    if http.agent_id.is_none() {
        if let Some(id) = agent_id_arg {
            http.agent_id = Some(Arc::new(id));
        }
    }
}

pub async fn call_unary<Req, Resp>(
    cli_config: &crate::Config,
    handle: &Handle,
    method: reqwest::Method,
    path: &str,
    body: Option<Req>,
    agent_id_arg: Option<String>,
) -> Result<(), crate::error::Error>
where
    Req: serde::Serialize + Send,
    Resp: serde::de::DeserializeOwned + serde::Serialize + Send + 'static,
{
    let (_client, mut config) = crate::config::read(cli_config).await?;
    let mut http = super::client::build_http_client(cli_config, &mut config);
    apply_agent_id_arg(&mut http, agent_id_arg);
    let response: Resp = http.send_unary(method, path, body).await?;
    Output::Notification(Notification { agent_id: None, value: objectiveai_sdk::cli::output::NotificationValue::other(&(response)) })
        .emit(handle)
        .await;
    Ok(())
}

/// Variant of [`call_unary`] for endpoints whose API handler returns
/// an empty body on success (2xx with no JSON). Emits a single
/// `null` notification so consumers still see a "done" line.
pub async fn call_unary_no_response<Req>(
    cli_config: &crate::Config,
    handle: &Handle,
    method: reqwest::Method,
    path: &str,
    body: Option<Req>,
    agent_id_arg: Option<String>,
) -> Result<(), crate::error::Error>
where
    Req: serde::Serialize + Send,
{
    let (_client, mut config) = crate::config::read(cli_config).await?;
    let mut http = super::client::build_http_client(cli_config, &mut config);
    apply_agent_id_arg(&mut http, agent_id_arg);
    http.send_unary_no_response(method, path, body).await?;
    Output::Notification(Notification {
        agent_id: None,
        value: objectiveai_sdk::cli::output::NotificationValue::other(&(serde_json::Value::Null)),
    })
    .emit(handle)
    .await;
    Ok(())
}

pub async fn call_streaming<Req, Chunk>(
    cli_config: &crate::Config,
    handle: &Handle,
    method: reqwest::Method,
    path: &str,
    body: Option<Req>,
    agent_id_arg: Option<String>,
) -> Result<(), crate::error::Error>
where
    Req: serde::Serialize + Send,
    Chunk: serde::de::DeserializeOwned + serde::Serialize + Send + 'static,
{
    let (_client, mut config) = crate::config::read(cli_config).await?;
    let mut http = super::client::build_http_client(cli_config, &mut config);
    apply_agent_id_arg(&mut http, agent_id_arg);
    let stream = http
        .send_streaming::<Chunk, _, _>(method, path.to_string(), body)
        .await?;
    let mut stream = std::pin::pin!(stream);
    while let Some(result) = stream.next().await {
        let chunk = result?;
        Output::Notification(Notification { agent_id: None, value: objectiveai_sdk::cli::output::NotificationValue::other(&(chunk)) })
            .emit(handle)
            .await;
    }
    Ok(())
}

