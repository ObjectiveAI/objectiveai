//! `objectiveai viewer send <path> <body>` — POST a JSON body to the
//! viewer's HTTP server and await the response.
//!
//! The cli treats the viewer as externally running. Resolves the viewer
//! address from the env (`VIEWER_ADDRESS`) first, then falls back to
//! `viewer.address` + `viewer.port` composed via the same `compose_url`
//! helper `api/client.rs` uses. Same chain for `VIEWER_SIGNATURE` →
//! `viewer.signature`.
//!
//! Bypasses the SDK's `http::viewer::Client` because that client is
//! fire-and-forget — this command needs a synchronous send so the caller
//! can see the viewer's response.

use objectiveai_sdk::cli::output::{Handle, Notification, Output};

#[derive(serde::Serialize)]
struct ViewerSendResult {
    status: u16,
    body: serde_json::Value,
}

pub async fn run(
    cli_config: &crate::Config,
    handle: &Handle,
    path: &str,
    body: &str,
) -> Result<(), crate::error::Error> {
    let fs_client = objectiveai_sdk::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let mut config = fs_client.read_config().await?;

    if !path.starts_with('/') {
        return Err(crate::error::Error::ViewerPathMissingSlash(path.to_string()));
    }
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| crate::error::Error::ViewerBodyJsonParse(e.to_string()))?;

    // Resolve viewer address: env → viewer.address+port composed via
    // compose_url (same scheme-normalization rules as api/client.rs).
    let address = std::env::var("VIEWER_ADDRESS")
        .ok()
        .or_else(|| {
            let viewer = config.viewer();
            crate::api::client::compose_url(viewer.get_address(), viewer.get_port())
        })
        .ok_or(crate::error::Error::ViewerAddressNotConfigured)?;

    // Resolve viewer signature: env → viewer.signature.
    let signature = std::env::var("VIEWER_SIGNATURE")
        .ok()
        .or_else(|| config.viewer().get_signature().map(String::from));

    do_post(&address, path, signature.as_deref(), &parsed, handle).await
}

async fn do_post(
    address: &str,
    path: &str,
    signature: Option<&str>,
    body: &serde_json::Value,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let url = format!("{}{}", address.trim_end_matches('/'), path);

    let http_client = reqwest::Client::new();
    let mut req = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(body);
    if let Some(sig) = signature {
        req = req.header("X-VIEWER-SIGNATURE", sig);
    }
    let response = req
        .send()
        .await
        .map_err(|e| crate::error::Error::ViewerSendHttp(e.to_string()))?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    let response_body = serde_json::from_str::<serde_json::Value>(&response_text)
        .unwrap_or_else(|_| serde_json::Value::String(response_text));

    if status.is_success() {
        Output::<ViewerSendResult>::Notification(Notification { agent_id: None,
            value: ViewerSendResult {
                status: status.as_u16(),
                body: response_body,
            },
        })
        .emit(handle)
        .await;
        Ok(())
    } else {
        Err(crate::error::Error::ViewerSendBadStatus {
            status: status.as_u16(),
            body: response_body.to_string(),
        })
    }
}
