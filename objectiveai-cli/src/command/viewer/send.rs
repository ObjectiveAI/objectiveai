//! `viewer send <path> <body>` — POST a JSON body to the viewer's HTTP
//! server and return its `(status, body)`. Bypasses the SDK's
//! fire-and-forget viewer client because callers want to see the
//! response synchronously.
//!
//! Address resolution: env `VIEWER_ADDRESS` first, then a composed URL
//! from on-disk `viewer.address` + `viewer.port`. Signature: env
//! `VIEWER_SIGNATURE` first, then `viewer.signature`. Non-2xx status
//! codes are not an error; the response is returned as-is with the
//! status set on the `Response`.

use objectiveai_sdk::cli::command::viewer::send::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request { path, body, .. } = request;

    if !path.starts_with('/') {
        return Err(Error::ViewerPathMissingSlash(path));
    }

    let config = ctx.filesystem.read_config().await?;
    let viewer = config.viewer();

    let address = std::env::var("VIEWER_ADDRESS")
        .ok()
        .or_else(|| crate::context::compose_url(viewer.get_address(), viewer.get_port()))
        .ok_or(Error::ViewerAddressNotConfigured)?;
    let signature = std::env::var("VIEWER_SIGNATURE")
        .ok()
        .or_else(|| viewer.get_signature().map(String::from));

    let url = format!("{}{}", address.trim_end_matches('/'), path);

    let http_client = reqwest::Client::new();
    let mut req = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);
    if let Some(sig) = signature {
        req = req.header("X-VIEWER-SIGNATURE", sig);
    }
    let response = req
        .send()
        .await
        .map_err(|e| Error::ViewerSendHttp(e.to_string()))?;

    let status = response.status().as_u16();
    let text = response.text().await.unwrap_or_default();
    let body = serde_json::from_str::<serde_json::Value>(&text)
        .unwrap_or_else(|_| serde_json::Value::String(text));

    Ok(Response { status, body })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::send as sdk;
    use objectiveai_sdk::cli::command::viewer::send::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::send as sdk;
    use objectiveai_sdk::cli::command::viewer::send::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
