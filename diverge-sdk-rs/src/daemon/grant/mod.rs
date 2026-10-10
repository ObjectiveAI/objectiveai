//! Grants: what a role allows, kind by kind.
//!
//! A GRANT is one permission over one kind of thing the daemon holds —
//! agents, agent templates, tools, tool templates, outgoing
//! providers, incoming credentials, daemons, accounts, roles,
//! volumes, the database — and a [role](crate::daemon::endpoints::roles) is a
//! list of them, held by
//! [accounts](crate::daemon::endpoints::accounts). On the wire a grant
//! is one object with one member, named for the kind, whose value is
//! the kind's permission: `{"agents":…}`, `{"providers_incoming":…}`.
//! Every kind's permission takes one of these shapes, and the shape is
//! the logic:
//!
//! - **To make.** A bare array of the kind's making actions — the ones
//!   that bring something into being: a create, an add, a connect —
//! `{"agents":["create"]}`. Holding the action
//! is the whole of it; nothing is judged but that it is held, since
//! there is nothing yet to judge it over.
//! - **Over what exists.** An object of `actions`, the kind's actions
//!   over what exists, and `within`, how far they reach:
//! `{"agents":{"actions":["get","message"],"within":{"all_tags":["crew"]}}}`.
//! `within` is the string `"any"`, every one of the kind the daemon
//! holds, or the kind's own list filter — the one its `list` request
//! takes, `agents::list`'s
//! [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter)
//! for agents, and so on — read as a TEST: a thing passes when it
//! passes every member the filter gives, and a filter with no member
//! passes everything.
//! - **Tagging.** For the kinds that carry tags, an object of
//!   `actions`, `tag` or `untag` or both, `within` as above, and
//! `tags`, which tags may be put on or taken off: `"any"`, or the list
//! of them.
//! `{"agents":{"actions":["tag"],"within":"any","tags":["crew","idle"]}}`.
//! - **Over the one thing.** For the database, of which the daemon
//!   serves exactly one, a bare array of its actions,
//! `{"postgres":["get","connections"]}`: there is nothing to name and
//! nothing to narrow, so holding the action is the whole of it, as for
//! making.
//!
//! Each shape is its own variant with its own members, and an object
//! that is none of them does not decode: an action of one shape among
//! another's — `tag` among the actions over what exists, `get` among
//! the tagging ones — is malformed, not refused.
//!
//! # How a request is judged
//!
//! Every request is an action of one kind and, when it acts on what
//! exists, names the things it acts on. The daemon collects every grant
//! of that action from every role the account holds, and the request is
//! allowed when any one of them allows it: holds the making action, or
//! reaches every thing the request names. No grant denies, so the union
//! is the whole rule, and the order of roles and of grants never
//! matters. An action no grant allows is refused, and the refusal is
//! the endpoint's `Forbidden`. A list is judged thing by thing: it
//! sends what the account's `list` grants over the kind reach and
//! nothing else, narrowed further by the request's own filter, and is
//! `Forbidden` only when no grant allows `list` over the kind at all. A
//! container created under no account holds no grant.
//!
//! Two actions name no endpoint of their own kind: `assign` over
//! accounts, naming the account as a container's `account` at an
//! [agent's](crate::daemon::endpoints::agents::create) or a
//! [tool's](crate::daemon::endpoints::tools::create) create or edit;
//! and `grant` over roles, naming the role in an account's `roles` at
//! the account's create or edit. A request that names several things — an account and its roles, a
//! tool and an agent — is allowed when every one of them is.
//!
//! Moving files is two-sided. A download takes `download` over its
//! source; an upload takes `upload` over the agent, the tool or the
//! volume it lands in; a transfer takes `transfer` over its source and
//! `upload` over the agent, the tool or the volume it lands in. A
//! grant reaches a container or a volume whole: which paths within it
//! may be read or written is not a grant's to narrow.
//!
//! Mounting is two-sided the same way: naming a volume in a container's
//! mounts — a volume mount of the provider it is pinned to, or a FUSE
//! mount of a file or a directory in it — at the container's create or
//! edit takes `mount` over the volume, beside the `create` or `edit`
//! grant over the container.
//!
//! # The actions are the shared ones
//!
//! Every kind's `Make` and `Over`, with [`Tagging`] and [`Within`], are
//! defined once in [`shared::permission`](crate::shared::permission)
//! and re-exported here: the grants a dependency tool template carries
//! are made of the same actions, reaching by tags alone, and a grant
//! here reaches by the kind's own filter. Every kind but the database
//! carries tags — agents, templates, tools, providers, credentials,
//! daemons, accounts, roles and volumes alike — so every kind but the database
//! has the tagging shape. A grant that names nothing becomes one of
//! these by [`From`]: the same actions, reaching by a filter of the
//! tags alone, which is how a dependency tool's requests are judged.

mod convert;
mod grant;

pub use grant::*;
pub use crate::shared::permission::{Tagging, Within};

pub mod agents;
pub mod agents_templates;
pub mod tools;
pub mod tools_templates;
pub mod providers_outgoing;
pub mod providers_incoming;
pub mod providers_daemons;
pub mod accounts;
pub mod roles;
pub mod volumes;
pub use crate::shared::permission::postgres;
