//! What a server's channel response frame carries on a container
//! serve's filetree channel.

/// One change on the served subtree, or the news that the tree cannot
/// be watched: the
/// [`volumes::serve`](crate::provider::endpoints::volumes::serve) filetree
/// frame, byte for byte — `0` a [`filetree`](crate::shared::filetree)
/// frame, `1` an error — relayed here from the container's proxy as
/// it came. See
/// [`Frame`](crate::provider::endpoints::volumes::serve::server::channel_response::filetree::Frame)
/// for the encoding and how the channel ends.
///
/// # The tree is the container's
///
/// Where a volume serve's stream shows the volume as that serve sees
/// it, this one shows the subtree as the container holds it: the
/// snapshot is the directory the request named, less what the proxy
/// leaves out of every tree, and the changes after it are every
/// change in the subtree — this serve's own asks, and whatever the
/// program running in the container does — as the proxy watches
/// them. A directory the proxy could not watch is in the snapshot
/// with `changes` false, and no change under it is sent.
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
