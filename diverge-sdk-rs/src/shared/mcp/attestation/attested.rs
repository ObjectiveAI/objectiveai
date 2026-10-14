//! What the stamp writes: the agent, or the tool.

use serde::Serialize;

use super::{Agent, Tool};

/// What [`attest`](crate::shared::mcp::attest) writes under the key:
/// the [`Agent`] on a request, the [`Tool`] on an answer — one or the
/// other, never a choice, since which side a `_meta` is on says which.
pub trait Attested: Serialize {}

impl Attested for Agent {}

impl Attested for Tool {}
