//! The server side of a watch: what a provider sends.
//!
//! [`response`] is [`filetree::response`](crate::filetree::response)
//! under a shorter name, not a copy of it. A watch's answer IS a
//! filetree stream — the snapshot, then one frame per change — and
//! describing it a second time here would be two definitions of one
//! thing, waiting to disagree.
//!
//! There is no `request`. A provider opens no channels of its own to
//! answer a watch; it has everything it needs from the name.

pub use crate::filetree::response;
