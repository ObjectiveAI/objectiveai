//! Synchronous-response HTTP client for the viewer's HTTP server.
//!
//! Bypasses the SDK's fire-and-forget viewer path because callers
//! (`viewer send`) want to see the response. Constructed in
//! [`crate::context::Context::viewer_client`], which resolves the
//! address (config `viewer.address`, else the `viewer spawn` flow's
//! published lock URL) and the signature (env `VIEWER_SIGNATURE`,
//! else config `viewer.signature`).

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct ViewerClient {
    http: reqwest::Client,
    /// Base URL, e.g. `http://127.0.0.1:51234`.
    address: String,
    /// `X-VIEWER-SIGNATURE` header value, when configured.
    signature: Option<String>,
}

impl ViewerClient {
    pub fn new(address: String, signature: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            address,
            signature,
        }
    }

    /// POST `body` as JSON to `{address}{path}`. `path` must start
    /// with `/` ([`Error::ViewerPathMissingSlash`]). A non-2xx status
    /// is NOT an error — the response is returned as-is; the body is
    /// parsed as JSON, falling back to `Value::String(raw_text)`.
    pub async fn send(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<(u16, serde_json::Value), Error> {
        if !path.starts_with('/') {
            return Err(Error::ViewerPathMissingSlash(path.to_string()));
        }
        let url = format!("{}{}", self.address.trim_end_matches('/'), path);
        let mut req = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(body);
        if let Some(sig) = &self.signature {
            req = req.header("X-VIEWER-SIGNATURE", sig);
        }
        let response = req
            .send()
            .await
            .map_err(|e| Error::ViewerSendHttp(e.to_string()))?;

        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or(serde_json::Value::String(text));
        Ok((status, body))
    }
}
