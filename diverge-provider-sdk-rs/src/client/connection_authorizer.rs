//! Whether a connector may join a container the caller runs.

use std::future::Future;

use crate::shared::containers::authorize;

/// The runner's answer to a provider asking whether a connector may
/// attach to a container the runner started.
///
/// The provider relays the connector's socket address — attested,
/// the provider saw it — and whatever authorization the connector
/// offered — asserted, the connector wrote it; see
/// [`Authorize`](authorize::request::Authorize) for why the two are
/// not equal. The runner answers yes or no, and the connect scope
/// opens or is refused on that. There is no error: a runner that
/// cannot decide has decided no.
pub trait ConnectionAuthorizer: Send + Sync {
    /// Judge one connector.
    fn authorize(
        &self,
        request: &authorize::request::Authorize,
    ) -> impl Future<Output = authorize::response::Frame> + Send;
}
