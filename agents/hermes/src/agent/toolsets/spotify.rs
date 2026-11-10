//! Spotify control.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Spotify control, with its arguments — back in the vocabulary,
/// because its only blocker was rotation and rotation is what a
/// rotating secret is for: Hermes refreshes the PKCE state on
/// effectively every Spotify tool call and rewrites the store each
/// time, so the login is fed in before the loop and its rotated
/// form is collected after. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on.
///
/// APPLICATION: the harness sets `HERMES_SPOTIFY_CLIENT_ID` to
/// [`client_id`](Self::client_id) in the gateway's process
/// environment, and writes the fed login document as the
/// `providers.spotify` entry of `$HERMES_HOME/auth.json` (Hermes
/// keeps this entry inert for provider selection by its own
/// design); after the gateway exits the entry is read back as the
/// run left it and handed to the proxy's collect.
///
/// SECRET: `SPOTIFY_OAUTH`, rotating — the login document, a JSON
/// object, exactly as Hermes keeps the entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Toolset {
    /// The caller's own Spotify app (PKCE — there is no secret,
    /// and Hermes ships no default app). Static and public, so a
    /// plain argument.
    pub client_id: String,
}
