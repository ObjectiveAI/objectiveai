//! Serving a volume: the FUSE asks, answered from it.
//!
//! A caller names a volume and holds the scope; for as long as it
//! does, it opens channels on the scope carrying the nine asks of
//! [`fuse`](crate::shared::containers::fuse) — a stat, a piece read, a
//! piece written, a truncate, a setattr, a listing, a removal, a
//! rename, a mkdir — with no mount id, since the scope is the volume,
//! and the provider answers each from the volume as a caller's own
//! server would answer a mount. Split by who SENDS, as everywhere
//! else: the ask and the stop are in [`client`], the answers in
//! [`server`].
//!
//! # What it is for
//!
//! A FUSE mount is served by whoever the provider running the
//! container asks: the caller. A caller that holds no storage of its
//! own bridges instead — a mount on one provider to a volume on
//! another, each ask relayed here and its answer relayed back — and
//! this scope is the far end of that bridge. The same vocabulary
//! throughout, so the bridge is mechanical.
//!
//! # The volume is mounted
//!
//! A serve takes the SHARED hold, the one a run takes: the volume is
//! mounted for the scope's life, any number of serves and runs may
//! hold it at once, and a stat, a read, a write, a filetree, an edit
//! or a delete of it is refused meanwhile. A volume held exclusively
//! when the serve asks is the serve refused.
//!
//! # The three modes
//!
//! A volume's [`Mode`](super::Mode) says what becomes of a mutation
//! and who may serve it. A persistent volume is changed in place, and
//! has one user at a time: a serve of it is refused while a running
//! container mounts it or another serve holds it. An ephemeral volume
//! is served on a layer of the serve's own — every serve starts from
//! the volume as it is, its mutations land in the layer and are
//! discarded at the finish, and the request's
//! [`overlay_disk`](client::request::Frame::overlay_disk) caps the
//! layer: a mutation past the cap answers an error and the serve
//! continues. A read-only volume answers every mutating ask
//! [`ReadOnly`](crate::shared::containers::fuse::ack::Frame::ReadOnly),
//! which the caller relays as a read-only filesystem, and every
//! immutable ask goes through. Ephemeral and read-only volumes may be
//! served on any number of scopes, beside any number of containers.

pub mod client;
pub mod server;
