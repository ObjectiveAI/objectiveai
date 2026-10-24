//! What the provider asks podman, read out of podman's answers.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use tokio::process::Command;

#[cfg(not(target_os = "linux"))]
use super::MACHINE;
use super::{command, podman};
use crate::host::tools::{Error, capture};

/// The label an image carries to be kept: present in the image's
/// config, with any value, it exempts the image from the cache's
/// eviction. Set with `LABEL diverge.network/keep=true` when the
/// image is built.
pub const KEEP_LABEL: &str = "diverge.network/keep";

/// The names an image pulled from the provider's own registry
/// carries in the cache: every name under this prefix is one, since
/// the registry listens on the loopback and nowhere else, and no
/// image the provider holds on its own account is named so. A
/// client image is built and pulled under
/// `127.0.0.1:<port>/<hash>/<name>`, and a caller's image for a run
/// under `127.0.0.1:<port>/<repository>/…`, which is how the cache's
/// lookup by digest tells the provider's own images from what it
/// holds for one identity and never answers one identity's image to
/// another.
pub const CLIENT_PREFIX: &str = "127.0.0.1:";

/// One image in podman's storage, as `podman image ls` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The image id, the full hexadecimal, as `podman pull` answers
    /// it too.
    pub id: String,
    /// Its size in BYTES, every layer counted, shared or not.
    pub size: u64,
    /// When the image was built, in seconds since the epoch.
    pub created: i64,
    /// Whether its config carries [`KEEP_LABEL`].
    pub keep: bool,
}

/// One row of `podman image ls --format json`.
#[derive(Deserialize)]
struct ListedRow {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Created")]
    created: i64,
    #[serde(rename = "Labels")]
    labels: Option<HashMap<String, String>>,
    #[serde(rename = "Names")]
    names: Option<Vec<String>>,
}

impl ListedRow {
    /// Whether the image is the provider's own: held under at least
    /// one name that is not under [`CLIENT_PREFIX`]. An image with
    /// no name at all is not known to be, and is passed over too.
    fn own(&self) -> bool {
        self.names
            .as_ref()
            .is_some_and(|names| names.iter().any(|name| !name.starts_with(CLIENT_PREFIX)))
    }
}

/// Every image in the store, once each: `podman image ls` lists an
/// image once per name it is held under, and the ids are
/// deduplicated here, so a count over the answer counts every image
/// once.
pub async fn images() -> Result<Vec<Listed>, Error> {
    let rows = listed(command(["image", "ls", "--no-trunc", "--format", "json"])).await?;
    Ok(once(rows))
}

/// The rows deduplicated by id.
fn once(rows: Vec<ListedRow>) -> Vec<Listed> {
    let mut images: Vec<Listed> = Vec::with_capacity(rows.len());
    for row in rows {
        if images.iter().all(|seen| seen.id != row.id) {
            images.push(Listed {
                id: row.id,
                size: row.size,
                created: row.created,
                keep: row.labels.is_some_and(|labels| labels.contains_key(KEEP_LABEL)),
            });
        }
    }
    images
}

/// The image in the store with `digest` among the manifest digests
/// podman recorded for it, under any name or none: its id, or `None`
/// where the store has no such image. `podman image ls --filter
/// digest=<digest>` matches every digest an image was pulled by, so
/// no name is needed; an image loaded into the store with no digest
/// recorded is not matched, which is podman's limit. An image pulled
/// from the provider's own registry — one named under
/// [`CLIENT_PREFIX`] and nothing else — is not matched either, nor
/// one with no name: it is held for the identity it was pulled for,
/// and the deployer finds it by that identity, not here.
pub async fn image_by_digest(digest: &str) -> Result<Option<String>, Error> {
    let filter = format!("digest={digest}");
    let rows = listed(command(["image", "ls", "--no-trunc", "--filter", &filter, "--format", "json"])).await?;
    Ok(rows.into_iter().find(ListedRow::own).map(|row| row.id))
}

/// A listing of images, as podman answers `command`, its rows read.
async fn listed(command: Command) -> Result<Vec<ListedRow>, Error> {
    let answer = capture("podman", command).await?;
    serde_json::from_str(&answer).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })
}

/// The id of the image `reference` names in the cache: the full
/// hexadecimal. A reference the cache does not hold is podman's
/// refusal.
pub async fn image_id(reference: &str) -> Result<String, Error> {
    let answer = capture("podman", command(["image", "inspect", "--format", "{{.Id}}", reference])).await?;
    Ok(answer.trim().to_string())
}

/// Whether the registry `reference` names serves its manifest:
/// `podman manifest inspect --authfile <file> docker://<reference>`,
/// with the credential the auth file holds for that registry. An
/// exit of `0` is yes and any other exit is no — a registry that
/// does not have it, one that will not say, one that cannot be
/// reached — since podman does not tell them apart. Only a podman
/// that could not be started is an error.
pub async fn manifest_exists(auth_file: &Path, reference: &str) -> Result<bool, Error> {
    let auth_file = auth_file.to_string_lossy().into_owned();
    let reference = format!("docker://{reference}");
    let finished = podman(["manifest", "inspect", "--authfile", &auth_file, &reference]).await?;
    Ok(finished.status.success())
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

/// The podman machine, as `podman machine inspect` describes it.
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// Its state, lowercase: `running`, `stopped`, or another word
    /// podman uses for a machine on its way between them.
    pub state: String,
    /// Whether podman inside it runs as root.
    pub rootful: bool,
    /// Its memory, in MiB, as podman counts it.
    pub memory: u64,
    /// The host directories it was made seeing, each at the same
    /// path inside; empty where podman lists none.
    pub mounts: Vec<std::path::PathBuf>,
    /// How it is reached over SSH.
    pub ssh: MachineSsh,
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
    /// The private key on this host that user is reached with: a
    /// file podman keeps under its data home, beside the machine's
    /// disk, so where it is says where podman keeps the machine.
    pub identity: std::path::PathBuf,
}

/// One machine of `podman machine inspect`, whose answer is a JSON
/// array of one. The disk is not reported, so none is read; the
/// mounts are reported where there are any.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct MachineRow {
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Rootful")]
    rootful: bool,
    #[serde(rename = "Resources")]
    resources: ResourcesRow,
    #[serde(rename = "Mounts", default)]
    mounts: Vec<MountRow>,
    #[serde(rename = "SSHConfig")]
    ssh: SshRow,
}

/// One host directory a machine mounts.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct MountRow {
    #[serde(rename = "Source")]
    source: std::path::PathBuf,
}

/// The `Resources` of one machine.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct ResourcesRow {
    #[serde(rename = "Memory")]
    memory: u64,
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

/// One row of `podman machine list`: the name is all that is read.
#[cfg(not(target_os = "linux"))]
#[derive(Deserialize)]
struct ListRow {
    #[serde(rename = "Name")]
    name: String,
}

/// The provider's machine, [`MACHINE`], or `None` where podman does
/// not list one of that name. Asked as two questions: the list,
/// which is empty rather than an error when there is no machine and
/// may name machines other programs made, and then the inspection
/// of the one by name — never of "the default", which podman
/// resolves to the same name and refuses as missing when a machine
/// of another name is the only one. The inspection is asked with no
/// `--format`: its plain answer is JSON, and `--format json`, which
/// the listing takes, is to the inspection a template that prints
/// the word.
#[cfg(not(target_os = "linux"))]
pub async fn machine() -> Result<Option<Machine>, Error> {
    let listed = capture("podman", command(["machine", "list", "--format", "json"])).await?;
    let rows: Vec<ListRow> = serde_json::from_str(&listed).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    if !rows.iter().any(|row| row.name == MACHINE) {
        return Ok(None);
    }
    let answer = capture("podman", command(["machine", "inspect", MACHINE])).await?;
    let mut rows: Vec<MachineRow> = serde_json::from_str(&answer).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    let row = rows.pop().ok_or_else(|| Error::Output {
        program: "podman".to_string(),
    })?;
    Ok(Some(Machine {
        state: row.state.to_lowercase(),
        rootful: row.rootful,
        memory: row.resources.memory,
        mounts: row.mounts.into_iter().map(|mount| mount.source).collect(),
        ssh: MachineSsh {
            port: row.ssh.port,
            user: row.ssh.user,
            identity: row.ssh.identity,
        },
    }))
}
