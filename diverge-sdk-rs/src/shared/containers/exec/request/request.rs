//! One command to run.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Run one command in the container, as [`exec`](super::super) states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub struct Request {
    /// The command and its arguments, run as they are, no shell
    /// between: the first item a path, or a name looked up on the
    /// container's `PATH`. At least one item.
    pub argv: Vec<String>,
    /// Where it runs, as path components from the container's root;
    /// absent, the container's own working directory (the image's
    /// `WORKDIR`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<Vec<String>>,
    /// Variables set for it, over the container's own environment;
    /// absent, none.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, String>,
}

impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
