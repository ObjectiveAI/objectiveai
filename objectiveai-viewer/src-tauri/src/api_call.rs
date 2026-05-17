//! Tauri command `api_call_run` — host-side dispatcher for
//! `api-call-invoke` postMessages from plugin iframes.
//!
//! The iframe's `ObjectiveAI` JS client (when constructed with
//! `viewer: true`) re-routes every HTTP method through a `postMessage`
//! → Tauri command → events back loop so it can call the upstream API
//! without dealing with CORS or auth-header plumbing. This command
//! consumes the postMessage and emits an
//! [`Event::ApiCall`](objectiveai_sdk::viewer::Event::ApiCall) stream
//! back to the originating iframe.
//!
//! Wire format on `value` field of each emitted Event::ApiCall:
//!
//! 1. `{"type":"begin"}` — emitted before any data.
//! 2. `{"type":"chunk","chunk":<body>}` — one per SSE event for
//!    streaming endpoints, one total for unary endpoints (carrying
//!    the full response body).
//! 3. `{"type":"error","error":<obj>}` — on dispatch / HTTP failure;
//!    replaces any further chunks.
//! 4. `{"type":"end"}` — emitted last, signals the iframe's
//!    AsyncIterable to terminate.

use futures::StreamExt;
use objectiveai_sdk::viewer::{
    ApiCallEnvelope, ApiCallSubType, Event, EventSender, HttpMethod,
};

/// Dispatch a single api-call-invoke request. Returns immediately
/// after spawning the worker; output flows back through `events_tx`.
#[tauri::command]
pub async fn api_call_run(
    events_tx: tauri::State<'_, EventSender>,
    http_client: tauri::State<'_, objectiveai_sdk::HttpClient>,
    sub_type: ApiCallSubType,
    body: serde_json::Value,
    origin: String,
) -> Result<(), String> {
    api_call_run_impl(
        events_tx.inner().clone(),
        http_client.inner().clone(),
        sub_type,
        body,
        origin,
    )
    .await
}

/// Tauri-free body of [`api_call_run`]. Lets integration tests
/// exercise the bridge without constructing a `tauri::State`. Same
/// fire-and-forget spawn semantics as the Tauri-wrapped form.
#[doc(hidden)]
pub async fn api_call_run_impl(
    events_tx: EventSender,
    http_client: objectiveai_sdk::HttpClient,
    sub_type: ApiCallSubType,
    body: serde_json::Value,
    origin: String,
) -> Result<(), String> {
    tokio::spawn(async move {
        run(http_client, events_tx, sub_type, body, origin).await;
    });
    Ok(())
}

async fn run(
    client: objectiveai_sdk::HttpClient,
    tx: EventSender,
    sub_type: ApiCallSubType,
    body: serde_json::Value,
    origin: String,
) {
    let emit = |envelope: ApiCallEnvelope| {
        let value = serde_json::to_value(envelope).unwrap_or(serde_json::Value::Null);
        let _ = tx.send(Event::ApiCall {
            destination: origin.clone(),
            sub_type: sub_type.clone(),
            value,
        });
    };

    emit(ApiCallEnvelope::Begin);

    match dispatch(&client, &sub_type, body, &emit).await {
        Ok(()) => {}
        Err(e) => {
            emit(ApiCallEnvelope::Error {
                error: serde_json::json!({"message": e.to_string()}),
            });
        }
    }

    emit(ApiCallEnvelope::End);
}

async fn dispatch<F>(
    client: &objectiveai_sdk::HttpClient,
    sub_type: &ApiCallSubType,
    body: serde_json::Value,
    emit: &F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(ApiCallEnvelope),
{
    let method = match sub_type.method() {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Delete => reqwest::Method::DELETE,
    };
    let path = sub_type.path();

    let streaming = matches!(sub_type.method(), HttpMethod::Post)
        && body
            .get("stream")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

    if streaming {
        let stream = client
            .send_streaming::<serde_json::Value, _, _>(method, path, Some(body))
            .await?;
        futures::pin_mut!(stream);
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => emit(ApiCallEnvelope::Chunk { chunk }),
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
        }
    } else {
        let body_opt = match sub_type.method() {
            HttpMethod::Get => None,
            _ => Some(body),
        };
        let response = client
            .send_unary::<serde_json::Value>(method, path, body_opt)
            .await?;
        emit(ApiCallEnvelope::Chunk { chunk: response });
    }
    Ok(())
}
