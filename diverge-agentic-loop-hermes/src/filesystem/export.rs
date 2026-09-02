//! What the filesystem streams back out.

/// One item of the way back up: a resource as the run left it, or a
/// piece of the continuation. The driver sends each as one container
/// frame — `Response::Resource`, `Response::Continuation` — and the
/// continuation's pieces come last, because the closer closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Export {
    /// A resource the request named, whole, as the run left it —
    /// under the request field's dotted path, which is the frame's
    /// name. The caller replaces what it holds.
    Resource {
        /// The request field's dotted path.
        name: &'static str,
        /// The resource's content now, whole.
        body: Vec<u8>,
    },
    /// One piece of the continuation, tagged, at most a piece's
    /// worth of a file behind its tag: see
    /// [`continuation::stream`](super::continuation::stream).
    Continuation(Vec<u8>),
}
