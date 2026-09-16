//! What the provider asks podman, read out of podman's answers.

use serde::Deserialize;

use super::command;
use crate::tools::{Error, capture};

/// One image in podman's storage, as `podman image ls` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The image id, the full hexadecimal, as `podman image inspect`
    /// reports it too.
    pub id: String,
    /// Its size in BYTES, every layer counted, shared or not.
    pub size: u64,
}

/// One row of `podman image ls --format json`.
#[derive(Deserialize)]
struct ListedRow {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Size")]
    size: u64,
}

/// Every image in the store, once each: `podman image ls` lists an
/// image once per name it is held under, and the ids are
/// deduplicated here, so a count over the answer counts every image
/// once.
pub async fn images() -> Result<Vec<Listed>, Error> {
    let answer = capture("podman", command(["image", "ls", "--no-trunc", "--format", "json"])).await?;
    let rows: Vec<ListedRow> = serde_json::from_str(&answer).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    let mut listed: Vec<Listed> = Vec::with_capacity(rows.len());
    for row in rows {
        if listed.iter().all(|seen| seen.id != row.id) {
            listed.push(Listed { id: row.id, size: row.size });
        }
    }
    Ok(listed)
}

/// The id of the image `reference` names in the store: the full
/// hexadecimal. A reference the store does not hold is podman's
/// refusal.
pub async fn image_id(reference: &str) -> Result<String, Error> {
    let answer = capture("podman", command(["image", "inspect", "--format", "{{.Id}}", reference])).await?;
    Ok(answer.trim().to_string())
}

/// The host port podman published the container's `inside` port to:
/// the port of the last `host:port` in `podman port`'s answer. An
/// answer with no port in it is [`Error::Output`].
pub async fn port(container: &str, inside: u16) -> Result<u16, Error> {
    let answer = capture("podman", command(["port", container, &format!("{inside}/tcp")])).await?;
    answer
        .lines()
        .filter_map(|line| line.trim().rsplit(':').next())
        .filter_map(|port| port.parse().ok())
        .last()
        .ok_or_else(|| Error::Output {
            program: "podman".to_string(),
        })
}

/// Every container, running or not, carrying `label`: the ids, one
/// per line as `podman ps` writes them.
pub async fn containers(label: &str) -> Result<Vec<String>, Error> {
    let answer = capture(
        "podman",
        command(["ps", "--all", "--quiet", "--no-trunc", "--filter", &format!("label={label}")]),
    )
    .await?;
    Ok(answer.lines().map(str::trim).filter(|id| !id.is_empty()).map(str::to_string).collect())
}

/// How the podman machine is reached over SSH, as `podman machine
/// inspect` reports it.
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSsh {
    /// The port on this host the machine's SSH listens at.
    pub port: u16,
    /// The user inside the machine.
    pub user: String,
    /// The private key on this host that user is reached with.
    pub identity: std::path::PathBuf,
}

/// One machine of `podman machine inspect --format json`.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct MachineRow {
    #[serde(rename = "SSHConfig")]
    ssh: SshRow,
}

/// The `SSHConfig` of one machine.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct SshRow {
    #[serde(rename = "IdentityPath")]
    identity: std::path::PathBuf,
    #[serde(rename = "Port")]
    port: u16,
    #[serde(rename = "RemoteUsername")]
    user: String,
}

/// The default machine's SSH settings. No machine is podman's
/// refusal.
#[cfg(not(target_os = "linux"))]
pub async fn machine_ssh() -> Result<MachineSsh, Error> {
    let answer = capture("podman", command(["machine", "inspect", "--format", "json"])).await?;
    let mut rows: Vec<MachineRow> = serde_json::from_str(&answer).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    let row = rows.pop().ok_or_else(|| Error::Output {
        program: "podman".to_string(),
    })?;
    Ok(MachineSsh {
        port: row.ssh.port,
        user: row.ssh.user,
        identity: row.ssh.identity,
    })
}
