//! A run, or a serve, refused because a volume it names is not in
//! the mode it meant.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::volumes::Mode;

/// The volume a request named under one mode that is in another, and
/// so refused the request: a run, whose every mount states the mode it
/// means the volume to have, or a serve, whose request does.
///
/// Nothing changed: the volume is in the mode reported here, as its
/// listing reports it, and only an
/// [`edit`](crate::provider::endpoints::volumes::edit) changes it. The
/// provider answers this after it has held the volume and given the
/// hold back, so that an edit cannot land between the look and the
/// answer — the first and only response of its scope, then the
/// finish — with nothing fetched and nothing deployed for it. The
/// mode the request meant is the request's to read back.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeMode {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
    /// The mode the volume is in.
    pub mode: Mode,
}
