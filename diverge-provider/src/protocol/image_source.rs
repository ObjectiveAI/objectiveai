//! A caller's image store, reached through its run scope.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;

use diverge_sdk::wire::server::answer::{Answer, answer};
use diverge_sdk::wire::server::answers::Answers;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::shared::containers::oci;

/// The caller's manifests and blobs, by digest, and its proof of
/// possession, asked for on the scope of a run or of an
/// [`images::push`](diverge_sdk::provider::endpoints::images::push):
/// what a provider takes an image from the caller with.
///
/// Each ask is one channel this end opens on the scope:
/// [`OciManifest`](diverge_sdk::provider::endpoints::containers::agents::run::server::channel_request::Frame::OciManifest)
/// answered by one frame, [`OciBlob`](diverge_sdk::provider::endpoints::containers::agents::run::server::channel_request::Frame::OciBlob)
/// by the blob's pieces until the finish, `OciProve` by one frame of
/// 32 bytes; an empty finish on any is a digest the caller does not
/// hold, which is `None` here. The provider verifies what it gets;
/// this fetches.
///
/// # Clone is a second handle to the same scope
///
/// A source asked after its scope is over gets `None` — the channel's
/// receiver closes without a finish — the honest answer for a caller
/// that is gone.
#[derive(Clone)]
pub struct ImageSource {
    scope: Arc<ScopeHandle>,
    manifest: fn(&str) -> Vec<u8>,
    blob: fn(&str) -> Vec<u8>,
    prove: fn(&oci::prove::request::Request) -> Vec<u8>,
}

/// A manifest the caller holds: its media type, for the registry's
/// `Content-Type`, and its bytes verbatim, which the digest is of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub media_type: String,
    pub body: Bytes,
}

impl ImageSource {
    /// From the scope and the endpoint's encodings of its three asks:
    /// both run families and a push carry the same three, as their own
    /// frame types.
    pub(crate) fn new(
        scope: Arc<ScopeHandle>,
        manifest: fn(&str) -> Vec<u8>,
        blob: fn(&str) -> Vec<u8>,
        prove: fn(&oci::prove::request::Request) -> Vec<u8>,
    ) -> Self {
        ImageSource { scope, manifest, blob, prove }
    }

    /// The caller's proof that it holds the scope's image, at the
    /// request's platform and keyed by its nonce:
    /// exactly one frame of 32 bytes, then the finish. `None` for the
    /// empty finish, a frame of any other length, more than one frame,
    /// and a caller that is gone — each a failed check.
    pub async fn prove(&self, request: &oci::prove::request::Request) -> Option<[u8; 32]> {
        let mut channel = self.scope.send_channel_request(&(self.prove)(request)).await;
        let mut frames = 0usize;
        let mut proof = None;
        while let Some(bytes) = channel.response_receiver.recv().await {
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => {
                    frames += 1;
                    proof = oci::prove::response::Frame::decode(&payload).ok().map(|frame| frame.proof);
                }
                // Read to the finish: a channel left mid-way keeps its
                // number for the connection's life.
                Some(Answer::Finish) => break,
                None => {}
            }
        }
        if frames == 1 { proof } else { None }
    }

    /// The manifest under `digest`, or `None` for one the caller does
    /// not hold — or a caller that is gone.
    pub async fn manifest(&self, digest: &str) -> Option<Manifest> {
        let mut channel = self.scope.send_channel_request(&(self.manifest)(digest)).await;
        let mut found = None;
        while let Some(bytes) = channel.response_receiver.recv().await {
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => {
                    if found.is_none() {
                        if let Ok(frame) = oci::manifest::response::Frame::decode(&payload) {
                            found = Some(Manifest {
                                media_type: frame.media_type.to_string(),
                                body: payload.slice_ref(frame.body),
                            });
                        }
                    }
                }
                // Read to the finish: a channel left mid-way keeps its
                // number for the connection's life.
                Some(Answer::Finish) => break,
                None => {}
            }
        }
        found
    }

    /// The blob under `digest` as its pieces in order, or `None` for
    /// one the caller does not hold — or a caller that is gone before
    /// the first piece. A stream that ends short — the caller going
    /// away mid-blob — is a blob the registry's hash rejects.
    pub async fn blob(&self, digest: &str) -> Option<BlobStream> {
        Answers::first(&self.scope, &(self.blob)(digest))
            .await
            .map(|answers| BlobStream { answers })
    }
}

impl std::fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageSource").field("scope", &self.scope).finish_non_exhaustive()
    }
}

/// A blob's pieces, in order, ending at the channel's finish.
///
/// Every frame's payload is a piece, verbatim — see
/// [`oci::blob`] — and there is
/// no marker for the last: the finish ends the stream, and the
/// receiver closing without one ends it short.
#[must_use = "a blob that is not polled is a channel nobody reads"]
#[derive(Debug)]
pub struct BlobStream {
    answers: Answers,
}

impl Stream for BlobStream {
    type Item = Bytes;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Bytes>> {
        // A channel that closed short ends the stream: the piece that
        // never came is what the registry's hash will miss.
        match ready!(Pin::new(&mut self.get_mut().answers).poll_next(cx)) {
            Some(Ok(piece)) => Poll::Ready(Some(piece)),
            Some(Err(_)) | None => Poll::Ready(None),
        }
    }
}
