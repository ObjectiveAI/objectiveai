//! The agent, registered once for the container's life — and made
//! runnable as it is registered.
//!
//! The server tells the container its agent exactly once, at
//! `POST /register`, before the first loop; the agent never changes
//! after. Registering is also the install: the harness and the
//! agent's source are written under [`DIR`], and its requirements are
//! pip-installed into the system interpreter, BEFORE the agent counts
//! as registered — so a run never meets a missing package, and an
//! agent whose requirements will not install is refused where the
//! caller can see why, and stays unregistered for a corrected one to
//! be told. Nothing here touches the proxy: the disk is the image's,
//! and pip reaches the index on its own.
//!
//! One registration at a time: the whole of [`register`] runs under a
//! lock, so two arriving together cannot interleave their installs,
//! and the second finds the agent set and is refused.

use std::sync::OnceLock;

use axum::http::StatusCode;
use tokio::sync::Mutex;

use crate::agent::Agent;

/// Where the agent lives on disk.
pub const DIR: &str = "/var/lib/diverge-python";

/// The harness, written from [`HARNESS_SOURCE`] at registration.
pub const HARNESS: &str = "/var/lib/diverge-python/harness.py";

/// The agent's source, written verbatim at registration.
pub const SOURCE: &str = "/var/lib/diverge-python/agent.py";

/// The harness, as the binary carries it.
const HARNESS_SOURCE: &str = include_str!("harness.py");

/// The agent, once registered.
static AGENT: OnceLock<Agent> = OnceLock::new();

/// The registration in progress, if one is.
static REGISTERING: Mutex<()> = Mutex::const_new(());

/// Register the agent: write it, install it, hold it.
///
/// `python3 -m pip install --no-cache-dir --break-system-packages`,
/// one `name<op>version` argument per requirement, waited for as long
/// as it takes — nothing here times out. `--break-system-packages`
/// because the interpreter is the system's, and the container is the
/// disposable environment Debian's PEP 668 guard exists to protect
/// elsewhere; the official python image's pip is not Debian's and
/// does not raise the guard, but the flag holds on either. No
/// requirements, no pip: the install is skipped entirely.
pub async fn register(agent: Agent) -> Result<(), Error> {
    let _registering = REGISTERING.lock().await;
    if AGENT.get().is_some() {
        return Err(Error::Registered);
    }

    tokio::fs::create_dir_all(DIR).await.map_err(Error::Files)?;
    tokio::fs::write(HARNESS, HARNESS_SOURCE)
        .await
        .map_err(Error::Files)?;
    tokio::fs::write(SOURCE, &agent.python)
        .await
        .map_err(Error::Files)?;

    if !agent.requirements.is_empty() {
        let output = tokio::process::Command::new("python3")
            .args([
                "-m",
                "pip",
                "install",
                "--no-cache-dir",
                "--break-system-packages",
            ])
            .args(agent.pip_arguments())
            .output()
            .await
            .map_err(Error::Pip)?;
        if !output.status.success() {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            return Err(Error::Requirements {
                status: output.status.code(),
                output: text,
            });
        }
    }

    // Checked empty above, under the lock this still holds: the set
    // cannot lose.
    let _ = AGENT.set(agent);
    Ok(())
}

/// Whether an agent is registered. The loop needs nothing of the
/// agent itself — its source is on disk, its packages installed —
/// only that it is there.
pub fn registered() -> bool {
    AGENT.get().is_some()
}

/// A registration that did not happen.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// One is registered already, and the agent never changes.
    #[error("the agent is registered, and it never changes")]
    Registered,
    /// The harness or the source could not be written.
    #[error("the agent's files could not be written: {0}")]
    Files(std::io::Error),
    /// pip could not be started at all.
    #[error("pip could not be started: {0}")]
    Pip(std::io::Error),
    /// pip ran and refused: the requirements did not install.
    #[error("the requirements did not install")]
    Requirements {
        /// pip's exit code, absent if a signal ended it.
        status: Option<i32>,
        /// Everything pip said, stdout then stderr.
        output: String,
    },
}

impl Error {
    /// The status the refusal carries: a second registration is a
    /// conflict, a requirement that will not install is the caller's
    /// mistake, and the disk or pip failing to start is the
    /// container's.
    pub fn status(&self) -> StatusCode {
        match self {
            Error::Registered => StatusCode::CONFLICT,
            Error::Files(_) | Error::Pip(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Requirements { .. } => StatusCode::BAD_REQUEST,
        }
    }

    /// The failure as JSON, the shape every refusal has.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Registered => serde_json::json!({
                "kind": "registered",
                "error": self.to_string(),
            }),
            Error::Files(_) => serde_json::json!({
                "kind": "files",
                "error": self.to_string(),
            }),
            Error::Pip(_) => serde_json::json!({
                "kind": "pip",
                "error": self.to_string(),
            }),
            Error::Requirements { status, output } => serde_json::json!({
                "kind": "requirements",
                "error": {
                    "status": status,
                    "output": output,
                },
            }),
        }
    }
}
