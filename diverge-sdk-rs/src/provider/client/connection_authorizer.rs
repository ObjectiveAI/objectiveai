//! Whether a connector may join a container the caller runs.

use std::future::Future;

use crate::shared::containers::authorize;

/// The runner's answer to a provider asking whether a connector may
/// attach to a container the runner started.
///
/// The provider relays the connector's socket address — attested,
/// the provider saw it — and whatever authorization the connector
/// offered — asserted, the connector wrote it; see
/// [`AuthorizeConnect`](authorize::request::AuthorizeConnect) for why the two are
/// not equal. The runner answers yes or no, and the connect scope
/// opens or is refused on that. There is no error: a runner that
/// cannot decide has decided no.
pub trait ConnectionAuthorizer: Send + Sync {
    /// Judge one connector.
    fn authorize_connect(
        &self,
        request: &authorize::request::AuthorizeConnect,
    ) -> impl Future<Output = authorize::response::Frame> + Send;

    /// Judge one lister: whether whoever a
    /// [`containers::tools::list_for`](crate::provider::endpoints::containers::tools::list_for)
    /// names this runner's identity for may be told of this
    /// container. Both of the lister's claims are the provider's own
    /// — the address it saw, the identity it authorized — so there is
    /// nothing asserted to doubt, only a peer to allow or not. A yes
    /// hands out the container's id and family, and nothing else;
    /// joining is a connector's own ask. A runner that cannot decide
    /// has decided no.
    fn authorize_list(
        &self,
        request: &authorize::request::AuthorizeList,
    ) -> impl Future<Output = authorize::response::Frame> + Send;
}
