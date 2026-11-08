//! Building an image into the Store, telling the build as it goes.

use diverge_sdk::shared::Identity;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::{Stream, stream};
use serde_json::Value;
use tokio::fs;
use tokio::sync::mpsc::{self, Sender};

use super::{Attempt, Error, Meta, Store, now};
use crate::host::container_deployer::{ContainerDeployer, descend, tool_path};
use crate::host::image_registry::Digest;
use crate::host::tools::{self, podman};
use crate::host::volume_manager::VolumeManager;
use crate::protocol::context::Context;
use crate::protocol::progress::Progress;
use crate::protocol::volume_manager::VolumeManager as _;
use diverge_sdk::provider::endpoints::images::build::server::response::{Built, Exists};
use diverge_sdk::shared::discover::Description;

/// The build as the manager hands it back: every line as it is
/// said, then how it ended.
pub type Build = Pin<Box<dyn Stream<Item = Result<Progress, Error>> + Send>>;

/// What one build needs, owned, since it runs on a task of its own.
struct Job {
    store: Arc<Store>,
    volumes: Arc<VolumeManager>,
    deployer: Arc<ContainerDeployer>,
    client_identity: Identity,
    description: Option<Description>,
    containerfile: String,
    context: Option<Context>,
}

/// The build started: a task of its own runs it and says what it
/// says on the stream handed back, and stops where the stream is
/// dropped, the tool killed with it.
///
/// In order:
///
/// 1. `run/builds/<uuid>/` made, with the Containerfile written into
///    it and, when the request named no context, an empty
///    `context/`; the context volume attached, when one was named,
///    and the directory descended.
/// 2. `podman build` in the cache, tagged
///    `127.0.0.1:<port>/build/<uuid>` — a name under the provider's
///    own registry, which the cache's lookup by digest passes over —
///    without layer caching so nothing dangles, pulling the `FROM` it
///    lacks through the cache's registries with the auth file; every
///    line it says sent. A failure is the error after the lines, with
///    nothing owned.
/// 3. The image exported once as an OCI layout and gathered into the
///    Store's blobs under an attempt — every blob pinned and moved
///    in, one the Store holds already left as it is, the manifest's
///    digest the image's from then on — and the build's tag removed
///    from the cache, whatever the end: a run pulls the image from
///    the Store like any other owned one.
/// 4. A digest the caller owns already is `Exists`, and nothing
///    changes.
/// 5. Every blob measured: over the capacity, cache records are
///    trimmed oldest first; still over, insufficient capacity, and
///    nothing owned.
/// 6. Else the ownership recorded, with the description and the
///    Containerfile, and the image built.
///
/// Whatever the end, the attempt leaves — the Store pruned of what it
/// brought in when nothing owns it — the volume is detached and the
/// build directory removed.
pub fn build(
    store: Arc<Store>,
    volumes: Arc<VolumeManager>,
    deployer: Arc<ContainerDeployer>,
    client_identity: &Identity,
    description: Option<&Description>,
    containerfile: &str,
    context: Option<&Context>,
) -> Build {
    let job = Job {
        store,
        volumes,
        deployer,
        client_identity: client_identity.clone(),
        description: description.cloned(),
        containerfile: containerfile.to_string(),
        context: context.cloned(),
    };
    // One step waits to be read at a time: the build's output is read
    // only as fast as its reader takes it.
    let (sender, receiver) = mpsc::channel(1);
    tokio::spawn(async move {
        if let Err(error) = job.run(&sender).await {
            let _ = sender.send(Err(error)).await;
        }
    });
    Box::pin(stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    }))
}

impl Job {
    /// The whole build; `Err` is the one error the stream ends with.
    async fn run(&self, sender: &Sender<Result<Progress, Error>>) -> Result<(), Error> {
        let dir = self.deployer.mounts_dir().with_file_name("builds").join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).await?;
        let outcome = self.in_dir(&dir, sender).await;
        let _ = fs::remove_dir_all(&dir).await;
        outcome
    }

    /// The build with its directory made: the context attached, the
    /// tool run, and the context detached after.
    async fn in_dir(&self, dir: &Path, sender: &Sender<Result<Progress, Error>>) -> Result<(), Error> {
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
    async fn podman(&self, containerfile: &Path, context: &str, sender: &Sender<Result<Progress, Error>>) -> Result<(), Error> {
        let reference = format!(
            "{}{}/build/{}",
            podman::CLIENT_PREFIX,
            self.deployer.registry_port(),
            uuid::Uuid::new_v4()
        );
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
        loop {
            // Nobody listens — the reader dropped the build, or never
            // will read again — the build is dropped, and its tool is
            // killed with it: heard as the reader goes, and not only
            // at the next line, since a tool can be silent for long.
            let line = tokio::select! {
                line = lines.next() => line,
                () = sender.closed() => return Ok(()),
            };
            let Some(line) = line else {
                break;
            };
            if sender.send(Ok(Progress::Output(line))).await.is_err() {
                return Ok(());
            }
        }
        let finished = pending.wait().await.map_err(Error::Podman)?;
        if !finished.status.success() {
            return Err(Error::Build(finished.status));
        }
        let attempt = self.store.enter().await;
        let outcome = self.owned(&reference, attempt, sender).await;
        self.untag(&reference).await;
        self.store.leave(attempt).await;
        let _ = self.store.prune().await;
        outcome
    }

    /// Steps 3 to 6, under `attempt`: the export gathered, and the
    /// ownership recorded or refused.
    async fn owned(&self, reference: &str, attempt: Attempt, sender: &Sender<Result<Progress, Error>>) -> Result<(), Error> {
        let digest = self.export(reference, attempt).await?;
        if self.store.owned(&self.client_identity, &digest).await?.is_some() {
            let _ = sender.send(Ok(Progress::Exists(Exists { digest: digest.to_string() }))).await;
            return Ok(());
        }
        if !self.store.make_room(0).await? {
            let _ = sender.send(Ok(Progress::InsufficientCapacity)).await;
            return Ok(());
        }
        let bytes = self.store.image_bytes(&digest).await;
        let meta = Meta {
            description: self.description.clone(),
            bytes,
            created: now(),
            containerfile: Some(self.containerfile.clone()),
        };
        if !self.store.own(&self.client_identity, &digest, &meta).await? {
            let _ = sender.send(Ok(Progress::Exists(Exists { digest: digest.to_string() }))).await;
            return Ok(());
        }
        let _ = sender.send(Ok(Progress::Built(Built {
            digest: digest.to_string(),
            bytes,
        }))).await;
        Ok(())
    }

    /// The image exported once into the Store: `podman save --format
    /// oci-dir` of the cache's copy into `tmp/<uuid>/`, written on
    /// this host by podman's client — which is what the host's podman
    /// is on macOS and Windows — and the layout gathered into the
    /// blobs under the attempt. The temporary directory is removed
    /// whatever happened.
    async fn export(&self, reference: &str, attempt: Attempt) -> Result<Digest, Error> {
        let tmp = self.store.tmp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(self.store.tmp_dir()).await?;
        let output = tmp.to_string_lossy().into_owned();
        let saved = podman::podman(["save", "--format", "oci-dir", "--output", &output, reference])
            .await
            .map_err(Error::Podman)
            .and_then(|finished| finished.require("podman", |status| status.success()).map_err(Error::Podman));
        let outcome = match saved {
            Ok(_) => gather(&self.store, attempt, &tmp).await,
            Err(error) => Err(error),
        };
        let _ = fs::remove_dir_all(&tmp).await;
        outcome
    }

    /// The build's tag removed from the cache, and the copy with it
    /// unless another name holds it. A refusal is not reported, since
    /// the build's own end is what is reported.
    async fn untag(&self, reference: &str) {
        let copy = podman::image_id(reference).await.ok();
        let _ = podman::podman(["rmi", reference]).await;
        if let Some(id) = &copy {
            if podman::image_id(id).await.is_err() {
                self.deployer.forget_image(id).await;
            }
        }
    }
}

/// The layout at `tmp` gathered into the Store: its `index.json`'s
/// one manifest read for the digest, and every file of its `blobs/`
/// pinned for the attempt and renamed into the Store's — the same
/// filesystem, so a rename — one the Store holds already left where
/// it is, which is a blob two images share stored once. Every blob's
/// digest is strict, and the manifest's is among them, or the layout
/// is [`Error::Layout`].
async fn gather(store: &Store, attempt: Attempt, tmp: &Path) -> Result<Digest, Error> {
    let digest = manifest_digest(tmp).await?;
    let digest = Digest::parse(&digest).ok_or_else(|| Error::Layout(tmp.to_path_buf()))?;
    let mut found = false;
    let mut algorithms = fs::read_dir(tmp.join("blobs")).await?;
    while let Some(algorithm) = algorithms.next_entry().await? {
        let Ok(algorithm_name) = algorithm.file_name().into_string() else {
            return Err(Error::Layout(tmp.to_path_buf()));
        };
        let mut files = fs::read_dir(algorithm.path()).await?;
        while let Some(file) = files.next_entry().await? {
            let Ok(hex) = file.file_name().into_string() else {
                return Err(Error::Layout(tmp.to_path_buf()));
            };
            let Some(blob) = Digest::parse(&format!("{algorithm_name}:{hex}")) else {
                return Err(Error::Layout(tmp.to_path_buf()));
            };
            found |= blob == digest;
            if store.hold(attempt, &blob).await.is_none() {
                let target = store.blob_path(&blob);
                if let Some(dir) = target.parent() {
                    fs::create_dir_all(dir).await?;
                }
                fs::rename(file.path(), &target).await?;
            }
        }
    }
    if !found {
        return Err(Error::Layout(tmp.to_path_buf()));
    }
    Ok(digest)
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
