//! What a run answers with: the container's [`Id`], or the
//! [`VolumeHeld`] or the [`VolumeMode`] that refused it.

mod id;
mod volume_held;
mod volume_mode;

pub use id::*;
pub use volume_held::*;
pub use volume_mode::*;
