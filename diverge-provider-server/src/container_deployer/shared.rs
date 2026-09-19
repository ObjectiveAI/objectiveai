//! What a deployer and every container it started hold between them.

use super::{Images, Limit};

/// The caps and the image cache: taken from by a deploy, given back
/// to by a container's stop, so both hold it.
#[derive(Debug)]
pub struct Shared {
    /// `container_overlay_disk` against the running containers'
    /// `disk`.
    pub disk: Limit,
    /// `memory` against the running containers' `memory`.
    pub memory: Limit,
    /// The image cache.
    pub images: Images,
}
