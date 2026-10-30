//! The daemon protocol: what a caller asks the Diverge daemon for.
//!
//! The daemon speaks the same frames, scopes and channels the provider
//! protocol does, over the same [`wire`](crate::wire) — its
//! [`frame`](crate::wire::frame),
//! [`connection`](crate::wire::connection),
//! [`encode`](crate::wire::encode) and [`decode`](crate::wire::decode),
//! the one error shape in [`shared::error`](crate::shared::error), and
//! the frame-level cores of each half — and defines its own
//! [`endpoints`] on top, as the [`provider`](crate::provider) defines
//! its own: agents, the tools attached to them, the templates both are
//! made from — one shape, [`template`], shared by the two families'
//! endpoints, as what their creates share is one [`create`](mod@create)
//! and what their edits share one [`patch`](mod@patch) — the providers
//! it dials and the credentials of
//! those that dial it, the volumes those providers hold, and the one
//! [database](endpoints::postgres) it serves into every container, its
//! own or a remote one. Who may ask for any of it is an
//! [account](endpoints::accounts), holding [roles](endpoints::roles) of
//! [grants](mod@grant). Who made each of those is [`creator`], carried
//! by every list item; how one list item points at another is [`key`],
//! and how a request names one of them, by name or once and for all, is
//! [`reference`](mod@reference). What a download sends, one piece of
//! one file with its path, is [`download`](mod@download)'s chunk, and
//! where a transfer lands is [`transfer`](mod@transfer)'s destination.
//! Every endpoint's exchange is performed by its `client::execute`, the
//! four shapes of which are [`client`]'s. The provider's endpoints are
//! not the daemon's, and nothing here names them but what a daemon
//! spawns an agent from.

/// The TCP port a daemon accepts the protocol on unless its own
/// configuration says otherwise: one above the port a provider
/// listens on, [`OUTSIDE_PORT`](crate::container_proxy::outside::OUTSIDE_PORT),
/// so that a daemon and a provider on one host do not contend for
/// it. What a client dials by default.
pub const DEFAULT_PORT: u16 = crate::container_proxy::outside::OUTSIDE_PORT + 1;

/// The file under a daemon's root — its configuration directory —
/// that a running daemon holds locked for its life, with its process
/// id inside: one daemon per root, a second start refused at once,
/// and the lock let go by the kernel when the holder dies. Named
/// here, beside the port, so a client that starts or stops a daemon
/// finds it where the daemon does; see
/// [`file_lock`](crate::file_lock).
pub const LOCK_FILE: &str = "daemon.lock";

pub mod client;
pub mod agent_log;
pub mod create;
pub mod creator;
pub mod download;
pub mod endpoints;
pub mod grant;
pub mod key;
pub mod patch;
pub mod reference;
pub mod template;
pub mod tool_log;
pub mod transfer;
