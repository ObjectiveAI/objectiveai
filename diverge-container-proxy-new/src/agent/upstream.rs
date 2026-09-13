//! The agent's server as a client, and the errors every call shares.

use diverge_container_proxy_sdk::agent;
use diverge_provider_sdk::shared::error::Error;

/// The HTTP client the five calls dial the agent's server with.
///
/// One client, so the connections it pools are shared; nothing else
/// is kept. The port is read from the environment on every call,
/// per the SDK's rule, so the number lives in one place. No timeouts
/// anywhere: an `/enqueue` is held until its fate, and a run's
/// stream is as long as the loop.
pub struct Upstream {
    http: reqwest::Client,
}

impl Upstream {
    pub fn new() -> Self {
        Upstream {
            http: reqwest::Client::new(),
        }
    }

    /// The client.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The agent's server's URL for one of its paths.
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", agent::port(), path)
    }
}

/// A non-`2xx`, as the wire's error: the body verbatim when it is
/// JSON — the agent's server speaking in the protocol's one error
/// shape — and wrapped, with the status, when it is not.
pub async fn status_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    let body = response.bytes().await.unwrap_or_default();
    match serde_json::from_slice(&body) {
        Ok(value) => Error(value),
        Err(_) => Error(serde_json::json!({
            "kind": "agent",
            "status": status,
            "error": String::from_utf8_lossy(&body),
        })),
    }
}

/// A call that never reached the agent's server, as the wire's error.
pub fn refused(error: &reqwest::Error) -> Error {
    Error(serde_json::json!({
        "kind": "agent",
        "error": error.to_string(),
    }))
}
