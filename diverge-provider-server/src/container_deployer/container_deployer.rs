//! The deployer.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use diverge_provider_sdk::server::container_deployer;
use diverge_provider_sdk::server::deployment::Deployment;
use futures_util::future;
use serde_json::json;

use super::deploy::deploy;
use super::{Container, Error, Images, Limit, Shared, Source, host_of, name_ok};
use crate::config::containers::{Containers, Podman};
use crate::tools::podman;
use crate::volume_manager::VolumeManager;

/// The provider's container deployer: podman, with the caps, the
/// image cache, the volumes, and the way to the registry.
#[derive(Debug)]
pub struct ContainerDeployer {
    /// The `podman` section: the registries and their credentials.
    podman: Podman,
    /// Every `(name, digest)` the configuration offers as a `server`
    /// image.
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
    /// provider's directory, `volumes` its volumes, and `registry`
    /// where its image registry listens. Made ready here: the proxy
    /// binary found beside the executable; `run/` and `run/mounts/`
    /// made and the auth file written; every container of an earlier
    /// life of this provider removed; the store's images recorded as
    /// protected; and, on the hosts with a machine, the tunnel
    /// opened. Each beside the others where nothing orders them.
    pub async fn new(
        containers: Containers,
        dir: PathBuf,
        volumes: Arc<VolumeManager>,
        registry: SocketAddr,
    ) -> Result<Self, Error> {
        let label = format!("diverge.provider={}", dir.display());
        let run_dir = dir.join("run");
        let mounts_dir = run_dir.join("mounts");
        tokio::fs::create_dir_all(&mounts_dir).await?;
        let auth_file = run_dir.join("auth.json");
        let (proxy_path, (), protected) = future::try_join3(
            proxy_beside_executable(),
            write_auth_file(&auth_file, &containers.podman),
            clean_then_list(&label),
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

    /// The port podman reaches the provider's registry at: the
    /// registry's own on Linux, where podman pulls on this host.
    #[cfg(target_os = "linux")]
    fn registry_port(&self, registry: SocketAddr) -> u16 {
        registry.port()
    }

    /// The port podman reaches the provider's registry at: the
    /// tunnel's, inside the machine, where podman pulls.
    #[cfg(not(target_os = "linux"))]
    fn registry_port(&self, _registry: SocketAddr) -> u16 {
        self.tunnel.port()
    }

    /// A registry reference as podman is handed it: one naming a
    /// listed host, as given; one naming no host, under the first
    /// listed host; one naming any other host, refused.
    fn registry_reference(&self, reference: &str) -> Result<String, Error> {
        match host_of(reference) {
            Some(host) => {
                if self.podman.registries.iter().any(|registry| registry.host == host) {
                    Ok(reference.to_string())
                } else {
                    Err(Error::Registry(host.to_string()))
                }
            }
            None => match self.podman.registries.first() {
                Some(registry) => Ok(format!("{}/{reference}", registry.host)),
                None => Err(Error::NoRegistry),
            },
        }
    }
}

impl container_deployer::ContainerDeployer for ContainerDeployer {
    type Container = Container;
    type Error = Error;

    /// The name checked as a repository path, then the image pulled
    /// from the provider's registry at the port podman reaches it by.
    async fn client(
        &self,
        _client_identity: &str,
        deployment: &Deployment,
        name: &str,
        digest: &str,
        registry: SocketAddr,
        repository: &str,
    ) -> Result<Container, Error> {
        if !name_ok(name) {
            return Err(Error::Name(name.to_string()));
        }
        let reference = format!("127.0.0.1:{}/{repository}/{name}@{digest}", self.registry_port(registry));
        deploy(self, deployment, Source::Client(reference)).await
    }

    /// The pair must be listed; then the store's own image, pulled
    /// from nowhere.
    async fn server(
        &self,
        _client_identity: &str,
        deployment: &Deployment,
        name: &str,
        digest: &str,
    ) -> Result<Container, Error> {
        if !self.offered.contains(&(name.to_string(), digest.to_string())) {
            return Err(Error::NotOffered {
                name: name.to_string(),
                digest: digest.to_string(),
            });
        }
        deploy(self, deployment, Source::Server(format!("{name}@{digest}"))).await
    }

    /// The reference under a listed host, pulled with that host's
    /// credential.
    async fn registry(
        &self,
        _client_identity: &str,
        deployment: &Deployment,
        reference: &str,
    ) -> Result<Container, Error> {
        let reference = self.registry_reference(reference)?;
        deploy(self, deployment, Source::Registry(reference)).await
    }
}

/// `diverge-container-proxy` beside the provider's executable, which
/// has to be there, as podman is handed it.
async fn proxy_beside_executable() -> Result<String, Error> {
    let executable = std::env::current_exe()?;
    let proxy = executable
        .parent()
        .map(|dir| dir.join("diverge-container-proxy"))
        .unwrap_or_else(|| PathBuf::from("diverge-container-proxy"));
    if tokio::fs::metadata(&proxy).await.is_err() {
        return Err(Error::Missing(proxy));
    }
    Ok(bind_path(&proxy))
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

/// Every container of an earlier life removed, then every image in
/// the store listed: the protected set, taken after the removal so a
/// container of the earlier life holds nothing back.
async fn clean_then_list(label: &str) -> Result<Vec<String>, Error> {
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
    let images = podman::images().await.map_err(Error::Podman)?;
    Ok(images.into_iter().map(|image| image.id).collect())
}

/// A host path as podman is handed it for a bind: on Linux, the path
/// itself.
#[cfg(target_os = "linux")]
fn bind_path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A host path as podman is handed it for a bind: the machine's view
/// of it.
#[cfg(not(target_os = "linux"))]
fn bind_path(path: &std::path::Path) -> String {
    podman::path(path)
}
