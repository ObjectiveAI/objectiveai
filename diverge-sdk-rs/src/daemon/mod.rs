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
//! and what their edits share one [`edit`](mod@edit) — the resources
//! served into them, the providers it dials and the credentials of
//! those that dial it, and the volumes those providers hold. Who may
//! ask for any of it is an [account](endpoints::accounts), holding
//! [roles](endpoints::roles) of [grants](mod@grant). Who made each of
//! those is [`creator`], carried by every list item; how one list item
//! points at another is [`key`], and how a request names one of them,
//! by name or once and for all, is [`reference`](mod@reference). What a
//! download sends, one piece of one file with its path, is
//! [`download`](mod@download)'s chunk, and where a transfer lands is
//! [`transfer`](mod@transfer)'s destination. Every endpoint's exchange
//! is performed by its `client::execute`, the four shapes of which are
//! [`client`]'s. The provider's endpoints are not the daemon's, and
//! nothing here names them but what a daemon spawns an agent from.

pub mod client;
pub mod create;
pub mod creator;
pub mod download;
pub mod edit;
pub mod endpoints;
pub mod grant;
pub mod key;
pub mod reference;
pub mod template;
pub mod transfer;
