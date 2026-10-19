//! The values, as the caller fed them.

use serde::{Deserialize, Serialize};

use crate::shared::containers::secrets::Values;

/// The body of `POST /secrets`: the [`Values`] themselves, one JSON
/// object of name to value, and nothing around them — every name the
/// program declared, in the order it declared them, a `fixed` one a
/// value read from its key and a `rotating` one the value locked to
/// the container until the next inactive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Request(pub Values);
