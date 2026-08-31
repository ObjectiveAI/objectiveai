//! Spotify control.

use serde::{Deserialize, Serialize};

/// The `spotify` switch.
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

/// Spotify control — back in the vocabulary, because its only
/// blocker was rotation and rotation is what resources exist for:
/// Hermes refreshes the PKCE state on effectively every Spotify
/// tool call and rewrites the store each time, so the caller's
/// state must flow in as a resource and its rotated form flow back.
///
/// APPLICATION: the harness sets `HERMES_SPOTIFY_CLIENT_ID` to
/// [`client_id`](Self::client_id) in the gateway's process
/// environment, fetches [`auth_resource`](Self::auth_resource)'s
/// bytes, and writes them as the `providers.spotify` entry of
/// `$HERMES_HOME/auth.json` (Hermes keeps this entry inert for
/// provider selection by its own design).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    /// The caller's own Spotify app (PKCE — there is no secret,
    /// and Hermes ships no default app). Static, so a plain
    /// argument.
    pub client_id: String,
    /// The OAuth state's size-bearing identity — the FILE grammar,
    /// `f1:<size>:<base64url sha256 of the bytes>` — never the
    /// bytes themselves. The bytes are the `providers.spotify`
    /// entry exactly as the caller's own Hermes stores it, JSON;
    /// they live with the client and arrive over the
    /// [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
    /// exchange when the provider does not hold them. In the
    /// document, `access_token` and `refresh_token` are the
    /// working pair; the run rotates them, and the rotated
    /// document is the resource's next state.
    pub auth_resource: String,
}
