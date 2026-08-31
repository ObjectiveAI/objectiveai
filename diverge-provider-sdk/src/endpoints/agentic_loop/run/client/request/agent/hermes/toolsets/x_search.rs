//! X (Twitter) search.

use serde::{Deserialize, Serialize};

/// The `x_search` switch.
///
/// Untagged: a bool is the bare switch, an object is the switch
/// thrown on with arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Toolset {
    /// `false` = explicitly off; `true` = on with nothing supplied.
    Switch(bool),
    /// On, with arguments. See [`Config`].
    Config(Config),
}

/// X (Twitter) search over the xAI API. The key path ONLY,
/// deliberately: Hermes's own dispatch prefers the key, and the
/// xAI subscription OAuth bearer answers this tool in a degraded
/// no-citation mode — the rotating path is a worse product here,
/// not a missing feature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    /// The xAI API key, applied as `XAI_API_KEY` in the gateway's
    /// process environment.
    pub api_key: String,
}
