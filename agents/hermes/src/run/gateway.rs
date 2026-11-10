//! The `hermes gateway` process: started, awaited, stopped.

use std::collections::BTreeMap;
use std::io;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::{Child, Command};

use super::Error;
use crate::filesystem::{API_SERVER_HOST, API_SERVER_PORT};

/// How often the readiness probe asks.
const PROBE: Duration = Duration::from_millis(250);

/// The gateway, alive as long as this is.
pub struct Gateway {
    /// The process. Killed on drop, so an unwinding runner leaves no
    /// gateway behind.
    child: Child,
}

impl Gateway {
    /// Spawn `hermes gateway` in the foreground, with exactly the
    /// environment [`prepare`](crate::filesystem::prepare) rendered
    /// on top of the container's own.
    pub fn start(env: &BTreeMap<String, String>) -> io::Result<Self> {
        let child = Command::new("hermes")
            .arg("gateway")
            .envs(env)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        Ok(Gateway { child })
    }

    /// Wait for the API server to answer `GET /health` — however
    /// long that takes; nothing here times out — failing only if the
    /// process exits first.
    pub async fn ready(&mut self) -> Result<(), Error> {
        let client = reqwest::Client::new();
        let health = format!("http://{API_SERVER_HOST}:{API_SERVER_PORT}/health");
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Err(Error::GatewayExited(status));
            }
            let up = client
                .get(&health)
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false);
            if up {
                return Ok(());
            }
            tokio::time::sleep(PROBE).await;
        }
    }

    /// Stop the gateway and wait for it: SIGTERM, which Hermes
    /// handles gracefully — the session finalized, the database
    /// closed — then the exit. Only after this may the database be
    /// folded.
    pub async fn stop(mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            if let Some(pid) = self.child.id() {
                // SAFETY: a plain signal to a pid this process
                // spawned and still owns.
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
            }
        }
        #[cfg(not(unix))]
        {
            // The check host is not the container; a hard kill is
            // all there is here, and it never runs.
            self.child.kill().await?;
        }
        self.child.wait().await?;
        Ok(())
    }
}
