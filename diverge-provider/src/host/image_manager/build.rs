//! Building an image into the store, telling the build as it goes.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::{Stream, stream};
use tokio::fs;
use tokio::sync::mpsc::{self, UnboundedSender};

use super::{Error, Meta, Store, name_ok_segment};
use crate::host::container_deployer::{ContainerDeployer, descend, tool_path};
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
    client_identity: String,
    name: String,
    containerfile: String,
    context: Option<Context>,
}

/// The build started: a task of its own runs it and says what it
/// says on the stream handed back, and stops where the stream is
/// dropped, the tool killed with it.
///
/// In order: the name checked and the identity's namespace locked;
/// the name found free; `run/builds/<uuid>/` made, with the
/// Containerfile written into it and, when the request named no
/// context, an empty `context/`; the context volume attached, when
/// one was named, and the directory descended; `podman build`
/// against the store's root, tagged for the identity and the name,
/// without layer caching so nothing dangles in the root, pulling the
/// `FROM` it lacks through the cache's registries with the auth file;
/// every line it says sent; then its end: a failure is the error
/// after the lines, with nothing built; a success is the image
/// inspected, the root measured, and — over the capacity — the image
/// removed again and insufficient capacity, or else the record
/// written and the image built. Whatever the end, the volume is
/// detached and the build directory removed.
pub fn build(
    store: Arc<Store>,
    volumes: Arc<VolumeManager>,
    deployer: Arc<ContainerDeployer>,
    client_identity: &str,
    name: &str,
    containerfile: &str,
    context: Option<&Context>,
) -> Build {
    let job = Job {
        store,
        volumes,
        deployer,
        client_identity: client_identity.to_string(),
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

    /// `podman build` against the root, read as it speaks, and its
    /// end judged.
    async fn podman(&self, containerfile: &Path, context: &str, sender: &UnboundedSender<Result<Progress, Error>>) -> Result<(), Error> {
        let tag = self.store.tag(&self.client_identity, &self.name);
        let file = tool_path(containerfile);
        let auth_file = tool_path(self.deployer.auth_file());
        let command = podman::command_in(
            self.store.root(),
            [
                "build",
                "--tag",
                &tag,
                "--file",
                &file,
                "--layers=false",
                "--pull=missing",
                "--authfile",
                &auth_file,
                context,
            ],
        );
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
        let inspected = podman::inspect_in(self.store.root(), &tag).await.map_err(Error::Podman)?;
        if self.store.used().await? > self.store.capacity() {
            let _ = podman::podman_in(self.store.root(), ["rmi", "--force", &tag]).await;
            let _ = sender.send(Ok(Progress::InsufficientCapacity));
            return Ok(());
        }
        let meta = Meta {
            digest: inspected.digest.clone(),
            bytes: inspected.size,
            created: now(),
            containerfile: self.containerfile.clone(),
        };
        self.store.write(&self.client_identity, &self.name, &meta).await?;
        let _ = sender.send(Ok(Progress::Built(Built {
            digest: inspected.digest,
            bytes: inspected.size,
        })));
        Ok(())
    }
}

/// Now, in seconds since the epoch.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
