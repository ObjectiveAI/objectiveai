//! An image's `/DIVERGE.md`, read out of the image and nothing run.

use diverge_sdk::shared::Identity;
use diverge_sdk::shared::containers::request::Image;

use super::deploy::obtain;
use super::tar::first_file;
use super::{ContainerDeployer, Error, name_ok};
use crate::host::tools::{self, capture_bytes, podman};
use crate::protocol::image_info::ImageInfo;

/// Where every image puts its document: the root of its filesystem.
const PATH: &str = "/DIVERGE.md";

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
/// 4. `podman cp` writes the file out as a tar stream on stdout, which
///    passes through the remote connection to a machine where a host
///    path would not. A cp that fails saying there is no such file is
///    [`ImageInfo::Missing`]; any other failure is [`Error::Podman`].
/// 5. The container is removed, whatever the cp answered.
/// 6. The stream's first entry, when it is a regular file, is
///    [`ImageInfo::Found`], its bytes; anything else is
///    [`ImageInfo::Missing`].
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
    podman::podman(["create", "--quiet", "--name", &name, "--label", &deployer.label, "--pull", "never", id, "true"])
        .await
        .and_then(|finished| finished.require("podman", |status| status.success()))
        .map_err(Error::Podman)?;
    let source = format!("{name}:{PATH}");
    let copied = capture_bytes("podman", podman::command(["cp", &source, "-"])).await;
    let _ = podman::podman(["rm", "--force", &name]).await;
    match copied {
        Ok(tar) => Ok(first_file(&tar).map_or(ImageInfo::Missing, |bytes| ImageInfo::Found(bytes.to_vec()))),
        Err(tools::Error::Status { stderr, .. }) if stderr.to_lowercase().contains("no such file") => Ok(ImageInfo::Missing),
        Err(error) => Err(Error::Podman(error)),
    }
}
