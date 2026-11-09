//! What a client can ask a provider for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`containers`] | run an agent or a tool server in a container; serve a subtree of a container live; read an image's DIVERGE.md without running it |
//! | [`images`] | list the images in the caller's Store, and keep the listing; examine one; build one; push one without building it; change one's description; destroy one; ask whether one image can be supplied, and pull its manifests, its blobs and its proof |
//! | [`volumes`] | list what a provider offers, and keep the listing; examine one; read, write or walk one; serve one's files live and watch them as served; ask how large one may be made; make one; ask how far one may grow; resize it; destroy it |
//! | [`daemons`] | accept connections from other daemons; connect to another daemon; list the connected identities that are listable, and keep the listing; listen for the messages sent to the caller, and keep listening; send one message to one identity |
//! | [`info`] | ask what a provider is: its version, and what it does |
//! | [`capacity`] | ask how much the caller may still use: room for a volume, room in the Store, memory and disk a run could reserve |
//!
//! # The tags
//!
//! Every request names itself with one byte at the front of its
//! payload. The frame layer never reads it — it carries one kind of
//! request frame and hands the bytes on — so this is the only thing
//! telling one request from another.
//!
//! | tag | request |
//! |-----|---------|
//! | `0` | [`containers::agents::run`] |
//! | `1` | [`containers::tools::run`] |
//! | `2` | [`containers::fuse_serve`] |
//! | `3` | [`containers::info`] |
//! | `4` | [`volumes::list`] |
//! | `5` | [`volumes::stat`] |
//! | `6` | [`volumes::read`] |
//! | `7` | [`volumes::write`] |
//! | `8` | [`volumes::filetree`] |
//! | `9` | [`volumes::fuse_serve`] |
//! | `10` | [`volumes::create_capacity`] |
//! | `11` | [`volumes::create`] |
//! | `12` | [`volumes::edit_capacity`] |
//! | `13` | [`volumes::edit`] |
//! | `14` | [`volumes::delete`] |
//! | `15` | [`images::list`] |
//! | `16` | [`images::stat`] |
//! | `17` | [`images::build`] |
//! | `18` | [`images::push`] |
//! | `19` | [`images::edit`] |
//! | `20` | [`images::delete`] |
//! | `21` | [`images::oci`] |
//! | `22` | [`info`] |
//! | `23` | [`capacity`] |
//! | `24` | [`daemons::accept`] |
//! | `25` | [`daemons::connect`] |
//! | `26` | [`daemons::list`] |
//! | `27` | [`daemons::listen`] |
//! | `28` | [`daemons::message`] |
//!
//! Twenty-nine, grouped by endpoint and ordered within it. The four
//! container scopes lead: the agents' run, the tools' run, the serve
//! that answers a running container's files live, and the info that
//! reads an image's DIVERGE.md without running it. The eleven volume
//! scopes follow in the
//! order a caller uses them: find one, examine it, read a file out of
//! it, write one in, see its tree, serve its files live, ask how large
//! one may be made, make one, ask how far one may grow, resize it,
//! destroy it.
//! Then the seven of [`images`]: the caller's Store — list, examine,
//! build, push, edit, destroy; and the oci scope of one image, which
//! asks whether the provider can supply it and, where it can, serves
//! its manifests, its blobs and its proof.
//! Then [`info`], which asks what a provider is, and [`capacity`],
//! which asks what is left of it. Last, the five of [`daemons`]: a
//! daemon accepting connections from other daemons, a daemon
//! connecting to one, the listing of which of the provider's connected
//! identities may be listed, a listen for the messages sent to the
//! caller, and a message to one identity.
//!
//! Nothing derives meaning from adjacency, which [`info`] is the
//! proof of — it is the one a client asks FIRST and it sits near the
//! end, because tags are handed out in the order scopes were defined
//! and nothing reads them in order. The grouping is for whoever reads
//! the table, and a new scope takes the value where it belongs
//! conceptually, the scopes after it moving up one, and a scope
//! removed takes its value with it, the scopes after it moving down —
//! which is how the container FUSE serve sits at `2`, the container
//! info at `3`, the volumes at `4` through `14`, the images at `15`
//! through `21` — the has and the hash that once stood among them
//! having gone into the oci scope — and [`capacity`] at `23` beside
//! [`info`], the daemons at `24` through `28`.
//!
//! This table is the whole allocation. Each request states its own
//! value and points here, because a value chosen in one module has to
//! be checked against every other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an
//! [`Invalid`](ClientRequest::Invalid) for a payload that is none of
//! them. It is the only place the values meet.
//!
//! # One substrate, two families
//!
//! [`containers`] is the substrate, and the two families under it are
//! told apart by what a caller says into the container: a message
//! for an agent, or MCP. Everything else a container scope carries —
//! the arguments a container is made with and the image's `info` that
//! says what they may be among it — is the same wire in both, defined
//! once in [`shared::containers`](crate::shared::containers). A
//! [`containers::fuse_serve`] reaches into either family's container the
//! same way, and carries the volume FUSE serve's own asks.
//!
//! # One open at a time
//!
//! Six scopes are held at most one of each per connection: the
//! whole-connection views [`volumes::list`], [`images::list`],
//! [`daemons::list`] and [`daemons::accept`], and the two messaging
//! scopes, [`daemons::listen`] and [`daemons::message`]. The rules:
//!
//! 1. **One at a time.** On one connection, at most one scope of each
//!    of the six is open at a time.
//! 2. **Refused, not queued.** A request opening a second while the
//!    first is open is answered with exactly one response, the
//!    endpoint's `AlreadyOpen`, and the finish follows it. Nothing else
//!    is done for it: nothing is listed, nothing is accepted, and
//!    nothing is sent.
//! 3. **Open until its finish.** A scope counts as open from the
//!    server reading its request until the server sends its finish.
//! 4. **Free before the finish.** The server lets the scope go before
//!    it sends the finish, so a client that reads the finish and opens
//!    another at once is never refused.
//! 5. **Per endpoint, per connection.** The six are independent of
//!    one another, and of every other connection. [`daemons::accept`]
//!    and [`daemons::listen`] are also one per identity: an accept or
//!    a listen under an identity that holds one on another connection
//!    is `AlreadyOpen` as well.

mod client_request;

pub use client_request::*;

pub mod capacity;
pub mod containers;
pub mod daemons;
pub mod images;
pub mod info;
pub mod volumes;
