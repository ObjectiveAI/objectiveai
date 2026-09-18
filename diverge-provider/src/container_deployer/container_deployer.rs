//! The deployer.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use diverge_provider_sdk::server::caller::Caller;
use diverge_provider_sdk::server::container_deployer;
use diverge_provider_sdk::server::deployment::Deployment;
use futures_util::future;
use serde_json::json;

use super::deploy::deploy;
use super::mounts::tool_path;
use super::source::find;
use super::{Container, Error, Images, Limit, Shared, Source, name_ok};
use crate::config::containers::{Containers, Podman};
use crate::tools::{mount, podman};
use crate::volume_manager::VolumeManager;

/// The provider's container deployer: podman, with the caps, the
/// image cache, the volumes, and the way to the registry.
#[derive(Debug)]
pub struct ContainerDeployer {
    /// The `podman` section: the registries and their credentials.
    pub(super) podman: Podman,
    /// Every `(name, digest)` the configuration lists as the
    /// provider's own: in the store already, the first place a run
    /// looks.
    offered: HashSet<(String, String)>,
    /// The proxy binary on this host, as podman is handed it for the
    /// bind: the host path on Linux, the machine's view elsewhere.
    pub(super) proxy_path: String,
    /// The auth file the provider wrote for podman, under `run/`.
    pub(super) auth_file: PathBuf,
    /// Where stored volumes are loop-mounted, `run/mounts/`, on this
    /// host.
    pub(super) mounts_dir: PathBuf,
    /// The label every container of this provider carries,
    /// `diverge.provider=<dir>`, for the cleanup of an earlier life.
    pub(super) label: String,
    /// The volumes, for the mounts.
    pub(super) volumes: Arc<VolumeManager>,
    /// The caps and the image cache, shared with every container.
    pub(super) shared: Arc<Shared>,
    /// The way into the podman machine, on the hosts with one.
    #[cfg(not(target_os = "linux"))]
    pub(super) tunnel: super::Tunnel,
}

impl ContainerDeployer {
    /// A deployer over the `containers` section, with `dir` the
    /// provider's directory, `volumes` its volumes, `registry` where
    /// its image registry listens, and `shares` the directories the
    /// volumes live in, which a podman machine must see beside `dir`
    /// and the executable's directory. Made ready here: on Linux the
    /// provider found to be root, and on the hosts with a machine the
    /// machine brought to what the configuration says, first, since
    /// everything after asks podman; the proxy
    /// binary found beside the executable; `run/` and `run/mounts/`
    /// made and the auth file written; every container and every
    /// loop mount of an earlier life of this provider swept away; the
    /// store's images recorded as protected; and, on the hosts with a
    /// machine, the tunnel opened. Each beside the others where
    /// nothing orders them.
    pub async fn new(
        containers: Containers,
        dir: PathBuf,
        volumes: Arc<VolumeManager>,
        registry: SocketAddr,
        shares: Vec<PathBuf>,
    ) -> Result<Self, Error> {
        let proxy = proxy_beside_executable()?;
        #[cfg(target_os = "linux")]
        {
            let _ = shares;
            super::root::ensure().await?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            let mut shares = shares;
            shares.push(dir.clone());
            if let Some(beside) = proxy.parent() {
                shares.push(beside.to_path_buf());
            }
            super::machine::ensure(containers.podman.memory, &containers.podman.storage_path, &shares).await?;
        }
        let label = format!("diverge.provider={}", dir.display());
        let run_dir = dir.join("run");
        let mounts_dir = run_dir.join("mounts");
        tokio::fs::create_dir_all(&mounts_dir).await?;
        let auth_file = run_dir.join("auth.json");
        let (proxy_path, (), protected) = future::try_join3(
            proxy_present(&proxy),
            write_auth_file(&auth_file, &containers.podman),
            sweep_then_list(&label, &mounts_dir),
        )
        .await?;
        #[cfg(not(target_os = "linux"))]
        let tunnel = super::Tunnel::open(&run_dir, registry).await?;
        #[cfg(target_os = "linux")]
        let _ = registry;
        Ok(ContainerDeployer {
            offered: containers
                .server_images
                .iter()
                .map(|image| (image.name.clone(), image.digest.clone()))
                .collect(),
            shared: Arc::new(Shared {
                disk: Limit::new(containers.podman.container_overlay_disk),
                memory: Limit::new(containers.podman.memory),
                images: Images::new(containers.podman.image_cache_disk, protected),
            }),
            podman: containers.podman,
            proxy_path,
            auth_file,
            mounts_dir,
            label,
            volumes,
            #[cfg(not(target_os = "linux"))]
            tunnel,
        })
    }

    /// Everything of this provider's that podman or the host still
    /// holds, gone: every container carrying its label removed at
    /// once, and every loop mount under `run/mounts/` unmounted and
    /// its directory removed. What the construction does before it
    /// counts the store, and what a shutdown does after the
    /// connections are gone. A refusal is not reported, since there
    /// is nobody to report it to.
    pub async fn sweep(&self) {
        let _ = sweep_all(&self.label, &self.mounts_dir).await;
    }

    /// The auth file the provider wrote for podman: what every pull
    /// and every look into a registry is given.
    pub fn auth_file(&self) -> &Path {
        &self.auth_file
    }

    /// The port podman reaches the provider's registry at: the
    /// registry's own on Linux, where podman pulls on this host.
    #[cfg(target_os = "linux")]
    pub(super) fn registry_port(&self, registry: SocketAddr) -> u16 {
        registry.port()
    }

    /// The port podman reaches the provider's registry at: the
    /// tunnel's, inside the machine, where podman pulls.
    #[cfg(not(target_os = "linux"))]
    pub(super) fn registry_port(&self, _registry: SocketAddr) -> u16 {
        self.tunnel.port()
    }
}

impl container_deployer::ContainerDeployer for ContainerDeployer {
    type Container = Container;
    type Error = Error;

    /// The name checked as a repository path; then the image found —
    /// in the store, when the configuration lists the pair; else in
    /// whichever of every listed registry and the caller first
    /// answers that it holds it — and run.
    async fn deploy(
        &self,
        _client_identity: &str,
        deployment: &Deployment,
        name: &str,
        digest: &str,
        caller: &Caller,
    ) -> Result<Container, Error> {
        if !name_ok(name) {
            return Err(Error::Name(name.to_string()));
        }
        let source = if self.offered.contains(&(name.to_string(), digest.to_string())) {
            Source::Server(format!("{name}@{digest}"))
        } else {
            find(self, name, digest, caller).await?
        };
        deploy(self, deployment, source).await
    }
}

/// Where `diverge-container-proxy` is: beside the provider's
/// executable.
fn proxy_beside_executable() -> Result<PathBuf, Error> {
    let executable = std::env::current_exe()?;
    Ok(executable
        .parent()
        .map(|dir| dir.join("diverge-container-proxy"))
        .unwrap_or_else(|| PathBuf::from("diverge-container-proxy")))
}

/// The proxy, which has to be there, as podman is handed it.
async fn proxy_present(proxy: &Path) -> Result<String, Error> {
    if tokio::fs::metadata(proxy).await.is_err() {
        return Err(Error::Missing(proxy.to_path_buf()));
    }
    Ok(tool_path(proxy))
}

/// The auth file, in the form podman reads: every listed registry
/// with a credential, as `user:password` in base64 under its host.
/// Readable by the provider alone where the filesystem has owners.
async fn write_auth_file(path: &std::path::Path, podman: &Podman) -> Result<(), Error> {
    let mut auths = serde_json::Map::new();
    for registry in &podman.registries {
        if let Some(credential) = &registry.credential {
            let pair = format!("{}:{}", credential.username, credential.password);
            auths.insert(registry.host.clone(), json!({ "auth": STANDARD.encode(pair) }));
        }
    }
    let document = serde_json::to_vec(&json!({ "auths": auths })).map_err(std::io::Error::other)?;
    tokio::fs::write(path, document).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    }
    Ok(())
}

/// Everything of an earlier life swept away, then every image in the
/// store listed: the protected set, taken after the sweep so a
/// container of the earlier life holds nothing back.
async fn sweep_then_list(label: &str, mounts_dir: &Path) -> Result<Vec<String>, Error> {
    sweep_all(label, mounts_dir).await?;
    let images = podman::images().await.map_err(Error::Podman)?;
    Ok(images.into_iter().map(|image| image.id).collect())
}

/// Every container carrying `label` removed at once, then every
/// directory under `mounts_dir` unmounted and removed, each beside
/// every other; a directory the tool would not unmount is removed
/// anyway where the host lets it go.
async fn sweep_all(label: &str, mounts_dir: &Path) -> Result<(), Error> {
    let orphans = podman::containers(label).await.map_err(Error::Podman)?;
    if !orphans.is_empty() {
        let mut args = vec!["rm", "--force", "--time", "0"];
        args.extend(orphans.iter().map(String::as_str));
        podman::podman(args)
            .await
            .map_err(Error::Podman)?
            .require("podman", |status| status.success())
            .map_err(Error::Podman)?;
    }
    let mut entries = tokio::fs::read_dir(mounts_dir).await?;
    let mut dirs = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        dirs.push(entry.path());
    }
    future::join_all(dirs.iter().map(|dir| async move {
        let _ = mount::unmount(&tool_path(dir)).await;
        let _ = tokio::fs::remove_dir(dir).await;
    }))
    .await;
    Ok(())
}
