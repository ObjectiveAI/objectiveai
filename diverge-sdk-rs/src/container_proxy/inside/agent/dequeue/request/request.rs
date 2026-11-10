//! What the program's slot is asked to give up.

use serde::{Deserialize, Serialize};

/// Empty the slot: `{}`, and nothing in it.
///
/// The proxy names no key, because the program holds no keys apart:
/// the slot is one message, the proxy's batch joined, and it is
/// withdrawn whole or not at all. Which of its messages the provider
/// meant to withdraw is the proxy's to remember; the rest it offers
/// again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {}
