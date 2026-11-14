//! What a client can ask a provider for.
//!
//! Each of these is one or more SCOPES — a request that opens one,
//! whatever channels either side needs inside it, and the answer that
//! comes back on channel `0`.
//!
//! | endpoint | scopes |
//! |----------|--------|
//! | [`containers`] | run an agent or a tool server in a container; serve a subtree of a container live |
//! | [`images`] | list the images in the caller's Store, and keep the listing; examine one; build one; push one without building it; change one's description; destroy one; ask whether one image can be supplied, and read it: its manifests, its blobs, its proof and its DIVERGE.md |
//! | [`volumes`] | list what a provider offers, and keep the listing; examine one; read, write or walk one; serve one's files live and watch them as served; ask how large one may be made; make one; ask how far one may grow; resize it; destroy it |
//! | [`daemons`] | accept connections from other daemons; connect to another daemon; list the connected identities that are announced, and keep the listing; listen for the messages sent to the caller, and keep listening; send one message to one identity; announce the caller to the listing, and keep it there |
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
//! | `3` | [`volumes::list`] |
//! | `4` | [`volumes::stat`] |
//! | `5` | [`volumes::read`] |
//! | `6` | [`volumes::write`] |
//! | `7` | [`volumes::filetree`] |
//! | `8` | [`volumes::fuse_serve`] |
//! | `9` | [`volumes::create_capacity`] |
//! | `10` | [`volumes::create`] |
//! | `11` | [`volumes::edit_capacity`] |
//! | `12` | [`volumes::edit`] |
//! | `13` | [`volumes::delete`] |
//! | `14` | [`images::list`] |
//! | `15` | [`images::stat`] |
//! | `16` | [`images::build`] |
//! | `17` | [`images::push`] |
//! | `18` | [`images::edit`] |
//! | `19` | [`images::delete`] |
//! | `20` | [`images::read`] |
//! | `21` | [`info`] |
//! | `22` | [`capacity`] |
//! | `23` | [`daemons::accept`] |
//! | `24` | [`daemons::connect`] |
//! | `25` | [`daemons::list`] |
//! | `26` | [`daemons::listen`] |
//! | `27` | [`daemons::message`] |
//! | `28` | [`daemons::announce`] |
//!
//! Twenty-nine, grouped by endpoint and ordered within it. The three
//! container scopes lead: the agents' run, the tools' run, and the
//! serve that answers a running container's files live. The eleven volume
//! scopes follow in the
//! order a caller uses them: find one, examine it, read a file out of
//! it, write one in, see its tree, serve its files live, ask how large
//! one may be made, make one, ask how far one may grow, resize it,
//! destroy it.
//! Then the seven of [`images`]: the caller's Store — list, examine,
//! build, push, edit, destroy; and the read scope of one image, which
//! asks whether the provider can supply it and, where it can, serves
//! its manifests, its blobs, its proof and its DIVERGE.md.
//! Then [`info`], which asks what a provider is, and [`capacity`],
//! which asks what is left of it. Last, the six of [`daemons`]: a
//! daemon accepting connections from other daemons, a daemon
//! connecting to one, the listing of which of the provider's connected
//! identities are announced, a listen for the messages sent to the
//! caller, a message to one identity, and the announce that puts the
//! caller in that listing.
//!
//! Nothing derives meaning from adjacency, which [`info`] is the
//! proof of — it is the one a client asks FIRST and it sits near the
//! end, because tags are handed out in the order scopes were defined
//! and nothing reads them in order. The grouping is for whoever reads
//! the table, and a new scope takes the value where it belongs
//! conceptually, the scopes after it moving up one, and a scope
//! removed takes its value with it, the scopes after it moving down —
//! which is how the container FUSE serve sits at `2`, the volumes at
//! `3` through `13` — the container info that once stood before them
//! having gone into the read scope — the images at `14` through `20`
//! — the has and the hash that once stood among them having gone
//! into the read scope too — and [`capacity`] at `22` beside
//! [`info`], the daemons at `23` through `28`.
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
//! the arguments a container is made with among it — is the same wire
//! in both, defined
//! once in [`shared::containers`](crate::shared::containers). A
//! [`containers::fuse_serve`] reaches into either family's container the
//! same way, and carries the volume FUSE serve's own asks.
//!
//! # One open at a time
//!
//! Seven scopes are held at most one of each per connection: the
//! whole-connection views [`volumes::list`], [`images::list`],
//! [`daemons::list`] and [`daemons::accept`], the two messaging
//! scopes, [`daemons::listen`] and [`daemons::message`], and
//! [`daemons::announce`]. The rules:
//!
//! 1. **One at a time.** On one connection, at most one scope of each
//!    of the seven is open at a time.
//! 2. **Refused, not queued.** A request opening a second while the
//!    first is open is answered with exactly one response, the
//!    endpoint's `AlreadyOpen`, and the finish follows it. Nothing else
//!    is done for it: nothing is listed, nothing is accepted, nothing
//!    is sent, and nothing is announced.
//! 3. **Open until its finish.** A scope counts as open from the
//!    server reading its request until the server sends its finish.
//! 4. **Free before the finish.** The server lets the scope go before
//!    it sends the finish, so a client that reads the finish and opens
//!    another at once is never refused.
//! 5. **Per endpoint, per connection.** The seven are independent of
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
