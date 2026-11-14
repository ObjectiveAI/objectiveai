//! Serving a volume over FUSE, through the daemon. One request, one
//! answer, and then a scope held open for the asks.
//!
//! The provider's
//! [`volumes::fuse_serve`](crate::provider::endpoints::volumes::fuse_serve),
//! asked of the daemon: a caller names a volume by its
//! [`reference::Volume`](crate::daemon::reference::Volume) — the
//! provider's identity and the name — and the mode it means it to be
//! in, and the daemon opens that provider's serve of it as itself.
//! The first response answers whether it is served. Where it is, the
//! scope stays open, and the caller opens on it the channels a
//! provider's serve answers: the nine asks a FUSE mount makes, the
//! file tree, and the cancel. The channel requests and their answers
//! are the provider's, byte for byte; the daemon carries each ask to
//! the provider's serve and each answer back, reading none of them.
//!
//! # What it is for
//!
//! A container reaching a volume that is not its own daemon's. A
//! daemon whose agent or tool mounts a file or a directory of a volume
//! on ANOTHER daemon opens this scope there, over its connection to
//! that daemon — relayed along a route where the volume's daemon is
//! further off, as any request's [`relay`](crate::shared::relay::Relay)
//! is — and answers its container's FUSE asks from it, as it answers
//! them from a provider's serve for a volume of its own. See
//! [`FuseMount`](crate::daemon::endpoints::agents::create::client::request::FuseMount).
//!
//! # The rules
//!
//! 1. **Its own grant.** The caller holds `fuse_serve` over the
//!    volume, an action of the volumes grant that no other action
//!    implies: not `mount`, which is naming a volume in a container of
//!    this daemon's own. Without it the answer is
//!    [`Forbidden`](server::response::Frame::Forbidden).
//! 2. **Exactly one answer, first.** `Serving`; one of the provider's
//!    refusals, relayed as the provider gave it — the volume mounted,
//!    served already, held, in another mode, or of a mode the provider
//!    does not serve; `NotFound`, for a provider the daemon keeps no
//!    record of or a volume it does not list; `Forbidden`; or an
//!    error. After any but `Serving` the finish follows at once, and
//!    no channel is answered.
//! 3. **The volume is in use while it is served.** From `Serving` to
//!    the scope's end, the daemon holds the volume as in use: a
//!    [`delete`](super::delete) of it answers `InUse`, and every
//!    request that takes a volume at rest answers `Held`, as they do
//!    while a running container has it.
//! 4. **The caller ends it.** The finish follows the caller's cancel,
//!    the caller gone, or the provider's serve ending. The provider's
//!    serve is cancelled with it.
//! 5. **A serve that ends early is a serve lost.** Where the
//!    provider's serve ends before the caller cancelled — the provider
//!    gone — the scope finishes, and a caller that mounted from it
//!    ends the run that did.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
