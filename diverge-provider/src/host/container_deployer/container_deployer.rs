//! The deployer.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use crate::protocol::caller::Caller;
use crate::protocol::container_deployer;
use crate::protocol::deployment::Deployment;
use futures_util::future;
use serde_json::json;

use super::deploy::deploy;
use super::mounts::tool_path;
use super::{Container, Error, Images, Shared, name_ok};
use crate::host::Limit;
use crate::host::image_manager::Store;
use crate::host::keys::Keys;
use crate::config::containers::{Containers, Podman};
use crate::host::tools::{mount, podman};
use crate::host::volume_manager::VolumeManager;
use diverge_sdk::shared::containers::request::Image;

/// The provider's container deployer: podman, with the caps, the
/// image cache, the volumes, and the way to the registry.
#[derive(Debug)]
pub struct ContainerDeployer {
    /// The `podman` section: the registries and their credentials.
    pub(super) podman: Podman,
    /// The proxy binary on this host, as podman is handed it for the
    /// bind: the host path on Linux, the machine's view elsewhere.
    pub(super) proxy_path: String,
    /// The auth file the provider wrote for podman, under `run/`.
    pub(super) auth_file: PathBuf,
    /// Where stored volumes are loop-mounted, `run/mounts/`, on this
    /// host.
    pub(super) mounts_dir: PathBuf,
    /// The client image store, where a caller's own images are looked
    /// for first; `None` where the configuration has none.
    pub(super) store: Option<Arc<Store>>,
    /// The port podman reaches the provider's registry at: the
    /// registry's own on Linux, where podman pulls on this host; the
    /// tunnel's elsewhere, inside the machine, where podman pulls.
    pub(super) registry_port: u16,
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
    /// its image registry listens, `shares` the directories the
    /// volumes live in, which a podman machine must see beside `dir`
    /// and the executable's directory, and `disk` the
    /// `container_overlay_disk` cap, which the volumes hold too, and
    /// `store` the client image store, if there is one. Made ready here: on Linux the
    /// provider found to be root, and on the hosts with a machine the
    /// machine brought to what the configuration says, first, since
    /// everything after asks podman; the proxy
    /// binary found beside the executable; `run/` and `run/mounts/`
    /// made and the auth file written; every container and every
    /// loop mount of an earlier life of this provider swept away;
    /// and, on the hosts with a machine, the tunnel opened. Each
    /// beside the others where nothing orders them.
    pub async fn new(
        containers: Containers,
        dir: PathBuf,
        volumes: Arc<VolumeManager>,
        registry: SocketAddr,
        shares: Vec<PathBuf>,
        disk: Arc<Limit>,
        store: Option<Arc<Store>>,
        keys: &Keys,
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
        let (proxy_path, (), ()) = future::try_join3(
            proxy_present(&proxy),
            write_auth_file(&auth_file, &containers.podman, keys),
            sweep_all(&label, &mounts_dir),
        )
        .await?;
        #[cfg(not(target_os = "linux"))]
        let tunnel = super::Tunnel::open(&run_dir, registry).await?;
        #[cfg(not(target_os = "linux"))]
        let registry_port = tunnel.port();
        #[cfg(target_os = "linux")]
        let registry_port = registry.port();
        Ok(ContainerDeployer {
            shared: Arc::new(Shared {
                disk,
                memory: Limit::new(containers.podman.memory),
                images: Images::new(containers.podman.image_cache_disk),
            }),
            podman: containers.podman,
            proxy_path,
            auth_file,
            mounts_dir,
            store,
            registry_port,
            label,
            volumes,
            #[cfg(not(target_os = "linux"))]
            tunnel,
        })
    }

    /// Everything of this provider's that podman or the host still
    /// holds, gone: every container carrying its label removed at
    /// once, and every loop mount under `run/mounts/` unmounted and
    /// its directory removed. What the construction does, and what a
    /// shutdown does after the connections are gone. A refusal is not
    /// reported, since there is nobody to report it to.
    pub async fn sweep(&self) {
        let _ = sweep_all(&self.label, &self.mounts_dir).await;
    }

    /// The auth file the provider wrote for podman: what every pull
    /// and every look into a registry is given.
    pub fn auth_file(&self) -> &Path {
        &self.auth_file
    }

    /// Where stored volumes are loop-mounted, `run/mounts/`: what a
    /// build attaches its context volume under, as a run does.
    pub fn mounts_dir(&self) -> &Path {
        &self.mounts_dir
    }

    /// Whether a container runs the image with `id` now.
    pub async fn running_image(&self, id: &str) -> bool {
        self.shared.images.running(id).await
    }

    /// The port podman reaches the provider's registry at: the
    /// registry's own on Linux, where podman pulls on this host; the
    /// tunnel's elsewhere, inside the machine, where podman pulls.
    /// What a client image is built under and pulled by.
    pub fn registry_port(&self) -> u16 {
        self.registry_port
    }

    /// What remains of the `memory` cap, in BYTES, as of now: what a
    /// run's `memory` could still be, reckoned against what the
    /// running containers reserved and not against the host's free
    /// memory.
    pub fn memory_room(&self) -> u64 {
        self.shared.memory.room()
    }

    /// What remains of the `container_overlay_disk` cap, in BYTES, as
    /// of now: what a run's `disk` could still be. The cap is the one
    /// the ephemeral FUSE serves' scratch counts against too, so what
    /// they hold is not room either.
    pub fn disk_room(&self) -> u64 {
        self.shared.disk.room()
    }
}

impl container_deployer::ContainerDeployer for ContainerDeployer {
    type Container = Container;
    type Error = Error;

    /// Every reference's name checked as a repository path; then the
    /// deploy — the image found and pulled beside the volumes bound,
    /// and the container run.
    async fn deploy(
        &self,
        client_identity: &str,
        deployment: &Deployment,
        image: &Image,
        caller: &Caller,
    ) -> Result<Container, Error> {
        if let Some(reference) = image.references.iter().find(|reference| !name_ok(&reference.name)) {
            return Err(Error::Name(reference.name.clone()));
        }
        deploy(self, client_identity, deployment, image, caller).await
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
/// with a credential, as `user:password` in base64 under its host,
/// the password the one its keyring entry keeps. Readable by the
/// provider alone where the filesystem has owners. The one file a
/// password is written to, since it is the one podman reads.
async fn write_auth_file(path: &std::path::Path, podman: &Podman, keys: &Keys) -> Result<(), Error> {
    let mut auths = serde_json::Map::new();
    for registry in &podman.registries {
        if let Some(credential) = &registry.credential {
            let password = keys
                .get(&credential.password)
                .ok_or_else(|| std::io::Error::other(format!("the keyring holds no `{}`", credential.password)))?;
            let pair = format!("{}:{}", credential.username, password.expose());
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
