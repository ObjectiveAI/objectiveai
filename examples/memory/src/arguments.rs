//! What the memory tool takes as its arguments: nothing.

use serde::{Deserialize, Serialize};

/// The arguments a memory tool is made with.
///
/// None: the notebook is the tool's database, and the tool is the
/// same whoever calls it. `{}` is the one value; any member is
/// refused, so a misspelled setting is never silently ignored.
///
/// # How it runs
///
/// As a dependency tool, declared by the image of the agent that
/// keeps the notes; meant to be transient. Whose notebook it is
/// follows from whose database the tool is given, never from anything
/// the tool decides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Arguments {}
