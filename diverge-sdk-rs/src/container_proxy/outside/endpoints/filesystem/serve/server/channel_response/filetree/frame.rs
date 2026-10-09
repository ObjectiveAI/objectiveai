//! What a server's channel response frame carries on a serve's
//! filetree channel.

/// One change on the served subtree, or the news that the tree cannot
/// be watched: the
/// [`volumes::serve`](crate::provider::endpoints::volumes::serve) filetree
/// frame, byte for byte — `0` a [`filetree`](crate::shared::filetree)
/// frame, `1` an error — which is what a provider relays to the
/// caller of a container serve as it came. See
/// [`Frame`](crate::provider::endpoints::volumes::serve::server::channel_response::filetree::Frame)
/// for the encoding and how the channel ends.
///
/// # The tree is the container's
///
/// The snapshot is the directory the request named, less what the
/// proxy leaves out of every tree, and the changes after it are every
/// change in the subtree — the serve's own asks, and whatever the
/// program beside the proxy does — watched as a
/// [`tree`](crate::container_proxy::outside::endpoints::filesystem::tree)
/// scope watches the container's root: the watch armed before the
/// walk, a directory the proxy could not watch in the snapshot with
/// `changes` false, and lost events answered by a fresh snapshot.
/// Every `path` is components from the subtree's root.
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::provider::endpoints::volumes::serve::server::channel_response::filetree::Frame;

/// A filetree response that could not be written. See
/// [`FrameEncodeError`](crate::provider::endpoints::volumes::serve::server::channel_response::filetree::FrameEncodeError).
pub type FrameEncodeError = crate::provider::endpoints::volumes::serve::server::channel_response::filetree::FrameEncodeError;

/// A filetree response that could not be read. See
/// [`FrameError`](crate::provider::endpoints::volumes::serve::server::channel_response::filetree::FrameError).
pub type FrameError = crate::provider::endpoints::volumes::serve::server::channel_response::filetree::FrameError;
