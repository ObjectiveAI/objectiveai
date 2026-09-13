//! Joining a container somebody else runs.

use serde::{Deserialize, Serialize};

/// Ask to join a container somebody else is running.
///
/// # What happens to the authorization
///
/// Nothing, here. The provider relays it to whoever holds the
/// container's run scope, as an
/// [`Authorize`](crate::shared::containers::authorize::request::Authorize),
/// and the answer to that is whether this scope opens.
///
/// Which is why the credential is opaque. A provider that had to
/// understand it would have to know what makes one connector
/// acceptable and another not, and it does not — the runner does, and
/// the runner is who reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Connect {
    /// The container to join.
    ///
    /// An [`Id`](crate::shared::containers::response::Id) from a run.
    /// It means nothing to a connector that was not given it, and
    /// nothing outside the provider that minted it.
    pub id: String,
    /// Whatever the runner needs in order to say yes.
    ///
    /// Opaque, and relayed verbatim. A shared secret, a signed token,
    /// a name — this layer does not know and does not look, so nothing
    /// here constrains what a runner chooses to require.
    ///
    /// Text, for the same reason an
    /// [`Auth`](crate::frame::auth::Auth) credential is: what this
    /// carries in practice already is a string, and bytes made a
    /// caller pick an encoding for something that never needed one.
    ///
    /// May be empty, which is a connector offering nothing. Whether
    /// that is ever enough is the runner's to decide.
    pub authorization: String,
}
