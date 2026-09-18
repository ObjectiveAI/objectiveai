//! What a run answers with: the container's [`Id`], or the
//! [`VolumeHeld`] that refused it.

mod id;
mod volume_held;

pub use id::*;
pub use volume_held::*;
