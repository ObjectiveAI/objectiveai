//! What the caller can supply for a run's image, handed to the
//! deployer.

use std::net::SocketAddr;
use std::sync::Arc;

use diverge_sdk::wire::server::answer::{Answer, answer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::shared::containers::oci;

/// The caller's help with one run's image, for the deployer to use or
/// ignore: whether the caller holds the image, asked on the run scope
/// by digest and answered with the repository path the caller holds
/// it under; and where the provider's own registry serves what the
/// caller holds, so a runtime can pull it.
///
/// A deployer that has the image already, or can pull it from a
/// registry it uses, need not touch this. One that would take the
/// image from the caller asks [`holds`](Self::holds) first, and then
/// points its runtime at `<registry>/<repository>/<name>@<digest>`,
/// with `name` the path the caller answered: the registry fetches
/// from the caller, by digest, whatever it does not hold, and
/// verifies what it gets. The registry is told to serve the
/// repository before the deploy and released after the run, whether
/// or not anything pulled; that is the handler's, not the deployer's.
///
/// # The answered name is the caller's
///
/// It lands in a reference by concatenation, so a deployer refuses
/// one that is not a repository path — treating it as not held — and
/// normalizes nothing. Nothing upstream of here does it.
///
/// # Clone is a second handle to the same scope
///
/// The scope outlives the deploy. A question asked after the run is
/// over is answered `None`: the channel's receiver closes without a
/// finish, which is the honest answer for a caller that is gone.
#[derive(Debug, Clone)]
pub struct Caller {
    scope: Arc<ScopeHandle>,
    has: fn(&str) -> Vec<u8>,
    digest: String,
    registry: SocketAddr,
    repository: String,
}

impl Caller {
    /// From the scope, the family's encoding of the has ask, the
    /// image's digest as the request named it, and where the registry
    /// serves this run's caller.
    pub(crate) fn new(
        scope: Arc<ScopeHandle>,
        has: fn(&str) -> Vec<u8>,
        digest: String,
        registry: SocketAddr,
        repository: String,
    ) -> Self {
        Caller {
            scope,
            has,
            digest,
            registry,
            repository,
        }
    }

    /// Whether the caller holds the image, and under what repository
    /// path: one channel on the run scope, one frame back. `None` for
    /// not held, an empty finish, a frame that does not decode, and a
    /// caller that is gone.
    pub async fn holds(&self) -> Option<String> {
        let mut channel = self.scope.send_channel_request(&(self.has)(&self.digest)).await;
        let mut held = None;
        while let Some(bytes) = channel.response_receiver.recv().await {
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => {
                    if let Ok(frame) = oci::has::response::Frame::decode(&payload) {
                        held = frame.name;
                    }
                }
                // Read to the finish: a channel left mid-way keeps its
                // number for the connection's life.
                Some(Answer::Finish) => break,
                None => {}
            }
        }
        held
    }

    /// Where the provider's registry listens, as a runtime pulls from
    /// it.
    pub fn registry(&self) -> SocketAddr {
        self.registry
    }

    /// The repository the registry serves this run's caller under:
    /// the image is `<registry>/<repository>/<name>@<digest>` there,
    /// with `name` the path the caller answered.
    pub fn repository(&self) -> &str {
        &self.repository
    }
}
