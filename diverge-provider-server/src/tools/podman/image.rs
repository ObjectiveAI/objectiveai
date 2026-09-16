//! What podman's store holds.

use super::podman;
use crate::tools::Error;

/// Whether podman holds the image `reference` names: `podman image
/// exists`, which exits `0` for an image in its store and `1` for one
/// not there, and prints nothing either way. Any other exit — a
/// reference podman cannot parse, a store it cannot open — is the
/// error.
///
/// The reference is whatever podman accepts: a name with or without
/// a registry host, with a tag or with `@<digest>`. A digest matches
/// the repo digests the image was pulled or loaded with, so
/// `docker.io/library/nginx@sha256:…` is found by the digest the
/// caller knows and not by any tag.
pub async fn image_exists(reference: &str) -> Result<bool, Error> {
    let finished = podman(["image", "exists", reference]).await?;
    match finished.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => finished.require("podman", |_| false).map(|()| false),
    }
}
