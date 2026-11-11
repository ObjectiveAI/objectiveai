//! Answering an approval prompt the instant it fires.

use crate::filesystem::{API_SERVER_HOST, API_SERVER_PORT};

/// Approve one pending request on a run: `POST /v1/runs/{run_id}/approval`
/// with `{"choice": "once"}`. The yolo trio makes prompts
/// unreachable; this is for the one that fires anyway, so the run
/// never eats the 300-second fail-closed stall. Fire and forget —
/// the outcome shows up on the stream as `approval.responded`, or
/// does not.
pub async fn approve(api_server_key: String, run_id: String) {
    let _ = reqwest::Client::new()
        .post(format!(
            "http://{API_SERVER_HOST}:{API_SERVER_PORT}/v1/runs/{run_id}/approval"
        ))
        .header("authorization", format!("Bearer {api_server_key}"))
        .json(&serde_json::json!({ "choice": "once" }))
        .send()
        .await;
}
