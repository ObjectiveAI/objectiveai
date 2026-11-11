//! What the chat takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments a chat tool is made with.
///
/// None: the chat is between a person and whichever agents are
/// attached to the tool, and the tool is the same whoever calls it.
/// `{}` is the one value; any member is refused, so a misspelled
/// setting is never silently ignored.
///
/// # How it runs
///
/// As a standalone tool, a service: it keeps the sockets of every open
/// page between the agents' calls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
