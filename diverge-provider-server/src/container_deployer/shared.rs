//! What a deployer and every container it started hold between them.

use std::sync::Arc;

use super::Images;
use crate::Limit;

/// The caps and the image cache: taken from by a deploy, given back
/// to by a container's stop, so both hold it.
#[derive(Debug)]
pub struct Shared {
    /// `container_overlay_disk` against the running containers'
    /// `disk` and the ephemeral serves' `overlay_disk`: the volumes
    /// hold the same one.
    pub disk: Arc<Limit>,
    /// `memory` against the running containers' `memory`.
    pub memory: Limit,
    /// The image cache.
    pub images: Images,
}
