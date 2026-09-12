//! What a run answers with: the container's [`Id`], or the
//! [`VolumeMounted`] that refused it.

mod id;
mod volume_mounted;

pub use id::*;
pub use volume_mounted::*;
