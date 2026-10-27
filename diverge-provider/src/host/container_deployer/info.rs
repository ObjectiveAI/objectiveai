//! An image's `/DIVERGE.md`, read out of the image and nothing run.

use std::path::Path;

use diverge_sdk::shared::Identity;
use diverge_sdk::shared::containers::request::Image;

use super::deploy::obtain;
use super::{ContainerDeployer, Error, name_ok};
use crate::host::tools::{self, podman};
use crate::protocol::image_info::ImageInfo;

/// Where every image puts its document: the root of its filesystem.
const PATH: &str = "/DIVERGE.md";

/// The name the file lands under, in the directory it is copied to.
const FILE: &str = "DIVERGE.md";

/// The image's `/DIVERGE.md`, as a deploy would find the image, with
/// no caller to ask.
///
/// The rules, in order:
///
/// 1. A reference whose name is not a repository path is
///    [`Error::Name`], as a deploy refuses one.
/// 2. The image is obtained as a deploy obtains it — the caller's own
///    pool, the cache by digest, a referenced registry the
///    configuration lists — and counted as starting in the cache for
///    the read, then as ended. One found nowhere is
///    [`ImageInfo::Unavailable`].
/// 3. A container is made of it and never started: named
///    `diverge-info-<uuid>`, labelled as the provider's so a sweep
///    finds one left behind, pulling nothing, with `true` as its
///    command, which is never run.
/// 4. `podman cp` copies the file into a directory of its own under the
///    deployer's run directory, `info/<container>`, named as the host
///    names it: a remote podman writes a copy to its client's
///    filesystem, so through a machine the path is the host's and not
///    the machine's view of it. Stdout is no destination, since a
///    remote podman on Windows looks for `/dev/stdout` on the host. A
///    cp that fails saying the path could not be found on the
///    container is [`ImageInfo::Missing`]; any other failure is
///    [`Error::Podman`].
/// 5. The container is removed, whatever the cp answered.
/// 6. What landed, when it is a regular file, is [`ImageInfo::Found`],
///    its bytes; anything else — a directory — is
///    [`ImageInfo::Missing`]. The directory is removed after.
pub(super) async fn read_info(deployer: &ContainerDeployer, client_identity: &Identity, image: &Image) -> Result<ImageInfo, Error> {
    if let Some(reference) = image.references.iter().find(|reference| !name_ok(&reference.name)) {
        return Err(Error::Name(reference.name.clone()));
    }
    let id = match obtain(deployer, client_identity, image, None).await {
        Ok((_, id)) => id,
        Err(Error::Unavailable { .. }) => return Ok(ImageInfo::Unavailable),
        Err(error) => return Err(error),
    };
    let read = read_from(deployer, &id).await;
    deployer.shared.images.ended(&id).await;
    read
}

/// Rules 3 to 6, of the image with `id`, which the store holds.
async fn read_from(deployer: &ContainerDeployer, id: &str) -> Result<ImageInfo, Error> {
    let name = format!("diverge-info-{}", uuid::Uuid::new_v4());
    let dir = deployer.mounts_dir.parent().unwrap_or(&deployer.mounts_dir).join("info").join(&name);
    tokio::fs::create_dir_all(&dir).await?;
    let read = copy_out(deployer, id, &name, &dir).await;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    read
}

/// Rules 3 to 6 within the directory `dir`, which the caller made and
/// removes.
async fn copy_out(deployer: &ContainerDeployer, id: &str, name: &str, dir: &Path) -> Result<ImageInfo, Error> {
    podman::podman(["create", "--quiet", "--name", name, "--label", &deployer.label, "--pull", "never", id, "true"])
        .await
        .and_then(|finished| finished.require("podman", |status| status.success()))
        .map_err(Error::Podman)?;
    let source = format!("{name}:{PATH}");
    let destination = dir.to_string_lossy().into_owned();
    let copied = podman::podman(["cp", source.as_str(), destination.as_str()])
        .await
        .and_then(|finished| finished.require("podman", |status| status.success()));
    let _ = podman::podman(["rm", "--force", name]).await;
    match copied {
        Ok(()) => {}
        Err(tools::Error::Status { stderr, .. }) if stderr.contains("could not be found on container") => return Ok(ImageInfo::Missing),
        Err(error) => return Err(Error::Podman(error)),
    }
    let landed = dir.join(FILE);
    match tokio::fs::metadata(&landed).await {
        Ok(metadata) if metadata.is_file() => Ok(ImageInfo::Found(tokio::fs::read(&landed).await?)),
        _ => Ok(ImageInfo::Missing),
    }
}
