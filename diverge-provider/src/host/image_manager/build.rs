//! Building an image into the pool, telling the build as it goes.

use diverge_sdk::shared::Identity;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{Stream, stream};
use serde_json::Value;
use tokio::fs;
use tokio::sync::mpsc::{self, UnboundedSender};

use super::{Error, Meta, Store, name_ok_segment};
use crate::host::container_deployer::{ContainerDeployer, descend, tool_path};
use crate::host::image_registry::Digest;
use crate::host::tools::{self, podman};
use crate::host::volume_manager::VolumeManager;
use crate::protocol::context::Context;
use crate::protocol::progress::Progress;
use crate::protocol::volume_manager::VolumeManager as _;
use diverge_sdk::provider::endpoints::images::build::server::response::Built;

/// The build as the manager hands it back: every line as it is
/// said, then how it ended.
pub type Build = Pin<Box<dyn Stream<Item = Result<Progress, Error>> + Send>>;

/// What one build needs, owned, since it runs on a task of its own.
struct Job {
    store: Arc<Store>,
    volumes: Arc<VolumeManager>,
    deployer: Arc<ContainerDeployer>,
    client_identity: Identity,
    name: String,
    containerfile: String,
    context: Option<Context>,
}

/// What an export put in the pool: the manifest digest the image is
/// known by from now on, every blob of it, and their size together.
struct Exported {
    digest: String,
    blobs: Vec<String>,
    bytes: u64,
}

/// The build started: a task of its own runs it and says what it
/// says on the stream handed back, and stops where the stream is
/// dropped, the tool killed with it.
///
/// In order: the name checked and the identity's namespace locked;
/// the name found free; `run/builds/<uuid>/` made, with the
/// Containerfile written into it and, when the request named no
/// context, an empty `context/`; the context volume attached, when
/// one was named, and the directory descended; `podman build` in the
/// cache, tagged with the name the image is pulled by from the
/// provider's own registry, without layer caching so nothing
/// dangles, pulling the `FROM` it lacks through the cache's
/// registries with the auth file; every line it says sent; then its
/// end: a failure is the error after the lines, with nothing built;
/// a success is the image exported once as an OCI layout into the
/// identity's pool — every blob moved in, one the pool holds already
/// left as it is, the manifest's digest the image's from then on —
/// every pool measured, and — over the capacity — the pool pruned
/// and the cache's copy removed again and insufficient capacity, or
/// else the record written and the image built, its copy left in
/// the cache for the run that follows. Whatever the end, the volume
/// is detached and the build directory removed.
pub fn build(
    store: Arc<Store>,
    volumes: Arc<VolumeManager>,
    deployer: Arc<ContainerDeployer>,
    client_identity: &Identity,
    name: &str,
    containerfile: &str,
    context: Option<&Context>,
) -> Build {
    let job = Job {
        store,
        volumes,
        deployer,
        client_identity: client_identity.clone(),
        name: name.to_string(),
        containerfile: containerfile.to_string(),
        context: context.cloned(),
    };
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        if let Err(error) = job.run(&sender).await {
            let _ = sender.send(Err(error));
        }
    });
    Box::pin(stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    }))
}

impl Job {
    /// The whole build; `Err` is the one error the stream ends with.
    async fn run(&self, sender: &UnboundedSender<Result<Progress, Error>>) -> Result<(), Error> {
        if !name_ok_segment(&self.name) {
            return Err(Error::Name(self.name.clone()));
        }
        let namespace = self.store.namespace(&self.client_identity);
        let _namespace = namespace.lock().await;
        if self.store.read(&self.client_identity, &self.name).await?.is_some() {
            return Err(Error::Exists(self.name.clone()));
        }
        let dir = self.deployer.mounts_dir().with_file_name("builds").join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).await?;
        let outcome = self.in_dir(&dir, sender).await;
        let _ = fs::remove_dir_all(&dir).await;
        outcome
    }

    /// The build with its directory made: the context attached, the
    /// tool run, and the context detached after.
    async fn in_dir(&self, dir: &Path, sender: &UnboundedSender<Result<Progress, Error>>) -> Result<(), Error> {
        let containerfile = dir.join("Containerfile");
        fs::write(&containerfile, &self.containerfile).await?;
        let (context, attached) = match &self.context {
            Some(context) => {
                let volume = self
                    .volumes
                    .get(&context.client_identity, &context.volume_name)
                    .await
                    .map_err(Error::Volumes)?
                    .ok_or_else(|| Error::Volume(context.volume_name.clone()))?;
                let base = volume.attach(self.deployer.mounts_dir()).await.map_err(Error::Mount)?;
                (descend(&base, &context.volume_relative_path), Some(volume))
            }
            None => {
                let empty = dir.join("context");
                fs::create_dir_all(&empty).await?;
                (tool_path(&empty), None)
            }
        };
        let outcome = self.podman(&containerfile, &context, sender).await;
        if let Some(volume) = attached {
            volume.detach().await;
        }
        outcome
    }

    /// `podman build` in the cache, read as it speaks, and its end
    /// judged.
    async fn podman(&self, containerfile: &Path, context: &str, sender: &UnboundedSender<Result<Progress, Error>>) -> Result<(), Error> {
        let reference = self.store.reference(self.deployer.registry_port(), &self.client_identity, &self.name);
        let file = tool_path(containerfile);
        let auth_file = tool_path(self.deployer.auth_file());
        let command = podman::command([
            "build",
            "--tag",
            &reference,
            "--file",
            &file,
            "--layers=false",
            "--pull=missing",
            "--authfile",
            &auth_file,
            context,
        ]);
        let (mut lines, pending) = tools::stream("podman", command).map_err(Error::Podman)?;
        while let Some(line) = lines.next().await {
            if sender.send(Ok(Progress::Output(line))).is_err() {
                // Nobody listens: the build is dropped, and its tool
                // is killed with it.
                return Ok(());
            }
        }
        let finished = pending.wait().await.map_err(Error::Podman)?;
        if !finished.status.success() {
            return Err(Error::Build(finished.status));
        }
        let exported = match self.export(&reference).await {
            Ok(exported) => exported,
            Err(error) => {
                self.undo(&reference).await;
                return Err(error);
            }
        };
        if self.store.used().await? > self.store.capacity() {
            self.undo(&reference).await;
            let _ = sender.send(Ok(Progress::InsufficientCapacity));
            return Ok(());
        }
        let meta = Meta {
            digest: exported.digest.clone(),
            bytes: exported.bytes,
            created: now(),
            containerfile: self.containerfile.clone(),
            blobs: exported.blobs,
        };
        if let Err(error) = self.store.write(&self.client_identity, &self.name, &meta).await {
            self.undo(&reference).await;
            return Err(error);
        }
        let _ = sender.send(Ok(Progress::Built(Built {
            digest: exported.digest,
            bytes: exported.bytes,
        })));
        Ok(())
    }

    /// The image exported once into the identity's pool: `podman
    /// save --format oci-dir` of the cache's copy into
    /// `<pool>/tmp/<uuid>/`, written on this host by podman's client
    /// — which is what the host's podman is on macOS and Windows —
    /// and the layout gathered into the pool. The temporary
    /// directory is removed whatever happened.
    async fn export(&self, reference: &str) -> Result<Exported, Error> {
        let pool = self.store.pool_dir(&self.client_identity);
        let tmp = pool.join("tmp").join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(pool.join("tmp")).await?;
        let outcome = self.save_into(reference, &tmp, &pool).await;
        let _ = fs::remove_dir_all(&tmp).await;
        outcome
    }

    /// The save and the gathering, with the temporary directory
    /// theirs.
    async fn save_into(&self, reference: &str, tmp: &Path, pool: &Path) -> Result<Exported, Error> {
        let output = tmp.to_string_lossy().into_owned();
        podman::podman(["save", "--format", "oci-dir", "--output", &output, reference])
            .await
            .map_err(Error::Podman)?
            .require("podman", |status| status.success())
            .map_err(Error::Podman)?;
        gather(tmp, pool).await
    }

    /// The cache's copy removed and the pool pruned: nothing of the
    /// build exists, since no record names its blobs. A refusal is
    /// not reported, since the build's own end is what is reported.
    async fn undo(&self, reference: &str) {
        let _ = podman::podman(["rmi", "--force", reference]).await;
        let _ = self.store.prune(&self.client_identity).await;
    }
}

/// The layout at `tmp` gathered into the pool: its `index.json`'s one
/// manifest read for the digest, and every file of its `blobs/`
/// renamed into the pool's — the same filesystem, so a rename — one
/// the pool holds already left where it is, which is a layer two of
/// the identity's images share stored once. Every blob's digest is
/// strict, and the manifest's is among them, or the layout is
/// [`Error::Layout`].
async fn gather(tmp: &Path, pool: &Path) -> Result<Exported, Error> {
    let digest = manifest_digest(tmp).await?;
    let mut blobs = Vec::new();
    let mut bytes = 0;
    let mut algorithms = fs::read_dir(tmp.join("blobs")).await?;
    while let Some(algorithm) = algorithms.next_entry().await? {
        let Ok(algorithm_name) = algorithm.file_name().into_string() else {
            return Err(Error::Layout(tmp.to_path_buf()));
        };
        let into = pool.join("blobs").join(&algorithm_name);
        fs::create_dir_all(&into).await?;
        let mut files = fs::read_dir(algorithm.path()).await?;
        while let Some(file) = files.next_entry().await? {
            let Ok(hex) = file.file_name().into_string() else {
                return Err(Error::Layout(tmp.to_path_buf()));
            };
            let blob = format!("{algorithm_name}:{hex}");
            if Digest::parse(&blob).is_none() {
                return Err(Error::Layout(tmp.to_path_buf()));
            }
            bytes += file.metadata().await?.len();
            let target = into.join(&hex);
            if fs::metadata(&target).await.is_err() {
                fs::rename(file.path(), &target).await?;
            }
            blobs.push(blob);
        }
    }
    if !blobs.contains(&digest) {
        return Err(Error::Layout(tmp.to_path_buf()));
    }
    Ok(Exported { digest, blobs, bytes })
}

/// The digest of the one manifest the layout's `index.json`
/// describes. A layout with no index, or an index that is not one
/// manifest's digest, is [`Error::Layout`].
async fn manifest_digest(layout: &Path) -> Result<String, Error> {
    let index: Value = serde_json::from_slice(&fs::read(layout.join("index.json")).await?)?;
    index
        .get("manifests")
        .and_then(|manifests| manifests.get(0))
        .and_then(|manifest| manifest.get("digest"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::Layout(layout.to_path_buf()))
}

/// Now, in seconds since the epoch.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
