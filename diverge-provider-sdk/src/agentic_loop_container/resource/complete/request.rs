//! The completion, which has nothing to say.

use serde::{Deserialize, Serialize};

/// The body of one `POST /resource/{identity}/complete`.
///
/// There is nothing to carry — the path names the resource and the
/// route IS the statement — but the route still gets a named body:
/// an empty object, `{}`, so the ask has a shape and a place for
/// one if a shape ever grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {}
