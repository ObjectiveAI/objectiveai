//! The agent, registered once for the container's life — and made
//! runnable as it is registered.
//!
//! The server tells the container its agent exactly once, at
//! `POST /register`, before the first loop; the agent never changes
//! after. Registering checks the bundle it names: `<bundle>/manifest.json`
//! must be there, because that file is what makes the directory a
//! tenant of `lone harness` rather than a path. The check happens
//! BEFORE the agent counts as registered, so a run never meets a
//! missing bundle, and an agent whose bundle is not there is refused
//! where the caller can see why, and stays unregistered for a
//! corrected one to be told. Nothing is installed and nothing is
//! written: the image carries the harness, and the bundle is mounted.
//! Nothing here touches the proxy.
//!
//! One registration at a time: the whole of [`register`] runs under a
//! lock, and the second finds the agent set and is refused.

use std::path::Path;
use std::sync::OnceLock;

use axum::http::StatusCode;
use tokio::sync::Mutex;

use crate::agent::Agent;

/// The file that makes a directory a bundle.
pub const MANIFEST: &str = "manifest.json";

/// The agent, once registered.
static AGENT: OnceLock<Agent> = OnceLock::new();

/// The registration in progress, if one is.
static REGISTERING: Mutex<()> = Mutex::const_new(());

/// Register the agent: check the bundle, hold the agent.
///
/// `manifest.json` is read as the existence check rather than parsed:
/// the credential-free shape `lone convert` writes is the harness's
/// to validate, and a bundle that is a directory with the file is a
/// bundle this program can be asked to run. A path without it is
/// `400 {kind: "bundle"}`, naming the directory.
pub async fn register(agent: Agent) -> Result<(), Error> {
    let _registering = REGISTERING.lock().await;
    if AGENT.get().is_some() {
        return Err(Error::Registered);
    }

    let manifest = Path::new(&agent.bundle).join(MANIFEST);
    match tokio::fs::metadata(&manifest).await {
        Ok(metadata) if metadata.is_file() => {}
        _ => return Err(Error::Bundle(agent.bundle)),
    }

    // Checked empty above, under the lock this still holds: the set
    // cannot lose.
    let _ = AGENT.set(agent);
    Ok(())
}

/// Whether an agent is registered.
pub fn registered() -> bool {
    AGENT.get().is_some()
}

/// The registered agent's turn invocation, or nothing before one is
/// registered.
pub fn arguments() -> Option<Vec<String>> {
    AGENT.get().map(Agent::arguments)
}

/// A registration that did not happen.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// One is registered already, and the agent never changes.
    #[error("the agent is registered, and it never changes")]
    Registered,
    /// The named directory is not a converted bundle.
    #[error("no bundle at this path: {0} has no {MANIFEST}")]
    Bundle(String),
}

impl Error {
    /// The status the refusal carries: a second registration is a
    /// conflict; a path that is not a bundle is the caller's mistake.
    pub fn status(&self) -> StatusCode {
        match self {
            Error::Registered => StatusCode::CONFLICT,
            Error::Bundle(_) => StatusCode::BAD_REQUEST,
        }
    }

    /// The failure as JSON, the shape every refusal has.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Registered => serde_json::json!({
                "kind": "registered",
                "error": self.to_string(),
            }),
            Error::Bundle(_) => serde_json::json!({
                "kind": "bundle",
                "error": self.to_string(),
            }),
        }
    }
}
