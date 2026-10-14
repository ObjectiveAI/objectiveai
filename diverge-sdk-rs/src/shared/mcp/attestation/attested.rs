//! What the stamp writes: the caller, or the tool.

use serde::Serialize;

use super::{Caller, Tool};

/// What [`attest`](crate::shared::mcp::attest) writes under the key:
/// the [`Caller`] on a request, the [`Tool`] on an answer — one or
/// the other, never a choice, since which side a `_meta` is on says
/// which. Each writes its `kind`; an [`Agent`](super::Agent) alone
/// would not, and is written as the caller it is.
pub trait Attested: Serialize {}

impl Attested for Caller {}

impl Attested for Tool {}
