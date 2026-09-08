//! The agent's schema: posted by the harness, answered to the server.

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::shared::containers::agent_schema::response;
use diverge_provider_sdk::shared::error::Error;
use futures_util::SinkExt as _;
use serde_json::Value;

/// The latest schema the harness posted, if any.
pub struct AgentSchema {
    value: Mutex<Option<Value>>,
}

impl AgentSchema {
    pub fn new() -> Self {
        AgentSchema {
            value: Mutex::new(None),
        }
    }
}

/// `POST /agent-schema/agent`: the harness says what its agent may
/// be. A later post replaces an earlier one.
pub async fn agent(
    State(schema): State<Arc<AgentSchema>>,
    Json(value): Json<Value>,
) -> StatusCode {
    match schema.value.lock() {
        Ok(mut slot) => {
            *slot = Some(value);
            StatusCode::NO_CONTENT
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// `/agent-schema`: the server asks. One message — the schema, or an
/// `Error` for an image that posted none — then the clean close.
pub async fn server(
    State(schema): State<Arc<AgentSchema>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, schema))
        .into_response()
}

async fn serve(mut socket: WebSocket, schema: Arc<AgentSchema>) {
    let posted = schema.value.lock().ok().and_then(|slot| slot.clone());
    let frame = match posted {
        Some(value) => response::Frame::AgentSchema(value),
        None => response::Frame::Error(Error(serde_json::json!({
            "kind": "agent_schema",
            "error": "the image posted no schema",
        }))),
    };
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok()
        && socket.send(Message::Binary(bytes.into())).await.is_ok()
    {
        let _ = socket.close().await;
    }
}
