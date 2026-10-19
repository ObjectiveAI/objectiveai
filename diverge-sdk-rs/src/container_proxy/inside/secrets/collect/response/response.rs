//! The values, as the program hands them back.

use serde::{Deserialize, Serialize};

use crate::shared::containers::secrets::Values;

/// The body of a `2xx` to `POST /secrets/collect`: the [`Values`]
/// themselves, one JSON object of name to value, and nothing around
/// them — every name the request asked for, each with the value the
/// program leaves in it, which may be the value it was fed. A name
/// asked for and absent here is the collect's error; a name present
/// and not asked for is ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Response(pub Values);
