//! Why the room could not be counted.

use std::fmt;

use crate::host::{image_manager, volume_manager};

/// What [`CapacityReporter`](super::CapacityReporter) fails with: a
/// store that could not be measured. The two container numbers never
/// fail, being counts held in memory.
#[derive(Debug)]
pub enum Error {
    /// The volume stores could not be measured.
    Volumes(volume_manager::Error),
    /// The client image store could not be measured.
    Images(image_manager::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Volumes(error) => write!(f, "the volume stores could not be measured: {error}"),
            Error::Images(error) => write!(f, "the client image store could not be measured: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Volumes(error) => Some(error),
            Error::Images(error) => Some(error),
        }
    }
}
