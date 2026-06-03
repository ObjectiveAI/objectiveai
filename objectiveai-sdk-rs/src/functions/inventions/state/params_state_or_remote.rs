use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A state specification that is either an inline ParamsState definition
/// or a remote path reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(
    rename = "functions.inventions.state.ParamsStateOrRemoteCommitOptional"
)]
pub enum ParamsStateOrRemoteCommitOptional {
    #[schemars(title = "ParamsState")]
    Inline(super::ParamsState),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePathCommitOptional),
}
