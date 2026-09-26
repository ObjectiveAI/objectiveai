//! Spotify control.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Spotify control, with its arguments — back in the vocabulary,
/// because its only blocker was rotation and rotation is what the
/// vault's lock exists for: Hermes refreshes the PKCE state on
/// effectively every Spotify tool call and rewrites the store each
/// time, so the login flows in from the vault and its rotated form
/// flows back. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on.
///
/// APPLICATION: the harness sets `HERMES_SPOTIFY_CLIENT_ID` to
/// [`client_id`](Self::client_id) in the gateway's process
/// environment; the login document lives in the vault under
/// [`SPOTIFY_OAUTH`](diverge_sdk::shared::containers::vault::keys::SPOTIFY_OAUTH), which the harness locks
/// for the run, reads, and writes as the `providers.spotify` entry
/// of `$HERMES_HOME/auth.json` (Hermes keeps this entry inert for
/// provider selection by its own design); at the run's end the
/// rotated document is set back under the key, and the key
/// unlocked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Toolset {
    /// The caller's own Spotify app (PKCE — there is no secret,
    /// and Hermes ships no default app). Static, so a plain
    /// argument.
    pub client_id: String,
}
