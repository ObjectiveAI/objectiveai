//! What the caller can supply for a run's image, handed to the
//! deployer.

use std::sync::Arc;

use diverge_sdk::wire::server::answer::{Answer, answer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;

use super::image_source::ImageSource;

/// The caller's help with one run's image, for the deployer to use or
/// ignore: whether the caller holds the image, asked on the run scope
/// by the image the run names — its digest and references — and
/// answered with a name; and the caller's store itself
/// — its manifests, its blobs and its proof of possession — as an
/// [`ImageSource`].
///
/// A deployer that has the image already, or can pull it from a
/// registry it uses, need not touch this. One that would take the
/// image from the caller asks [`holds`](Self::holds) first, fetches
/// through [`source`](Self::source) what it lacks, verifying every
/// byte against its digest, and ends with the nonce check of
/// [`prove`](diverge_sdk::shared::containers::oci::prove), always,
/// whether or not it fetched anything: only a pass makes the caller
/// the image's source.
///
/// # The answered name is a label
///
/// It is the caller's, and lands in a reference by concatenation, so
/// a deployer refuses one that is not a repository path — taking a
/// fixed label instead — and normalizes nothing. It is never stored,
/// and never read back.
///
/// # Clone is a second handle to the same scope
///
/// The scope outlives the deploy. A question asked after the run is
/// over is answered `None`: the channel's receiver closes without a
/// finish, which is the honest answer for a caller that is gone.
#[derive(Debug, Clone)]
pub struct Caller {
    scope: Arc<ScopeHandle>,
    has: fn(&Image) -> Vec<u8>,
    image: Image,
    source: ImageSource,
}

impl Caller {
    /// From the scope, the family's encoding of the has ask, the
    /// image as the request named it, and the caller's store over the
    /// same scope.
    pub(crate) fn new(scope: Arc<ScopeHandle>, has: fn(&Image) -> Vec<u8>, image: Image, source: ImageSource) -> Self {
        Caller {
            scope,
            has,
            image,
            source,
        }
    }

    /// The image the run names: what the has ask carries, and what
    /// the nonce check's request names.
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// Whether the caller holds the image, and under what name: one
    /// channel on the run scope, one frame back. `None` for not held,
    /// an empty finish, a frame that does not decode, and a caller
    /// that is gone.
    pub async fn holds(&self) -> Option<String> {
        let mut channel = self.scope.send_channel_request(&(self.has)(&self.image)).await;
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

    /// The caller's store: its manifests, its blobs and its proofs.
    pub fn source(&self) -> &ImageSource {
        &self.source
    }
}
