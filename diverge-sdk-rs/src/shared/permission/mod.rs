//! Permissions that name nothing: the grants a dependency tool
//! template asks for its tool, and the actions every grant of the
//! daemon's is made of.
//!
//! A GRANT is one permission over one kind of thing the daemon holds
//! — agents, agent templates, tools, tool templates, outgoing
//! providers, incoming credentials, accounts, roles, volumes, the
//! database — and on the wire it is one object with one member, named
//! for the kind, whose value is the kind's permission: `{"agents":…}`,
//! `{"volumes":…}`. The shapes are the daemon's own, stated in
//! [`daemon::grant`](crate::daemon::grant) — to make, over what
//! exists, tagging, the one thing — with one restriction, which is
//! what makes these shareable:
//!
//! # Nothing is named
//!
//! A grant here reaches what exists by TAGS alone. Where the daemon's
//! `within` is the kind's own list filter — names, templates, creators,
//! attachments — this one is [`Within`]: the string `"any"`, every one
//! of the kind, or a [`Tags`] test, those carrying every one of some
//! tags and any one of others. A tagging grant likewise touches `"any"`
//! tag or a listed set. No name of an agent, a tool, a volume, an
//! account or a role appears anywhere, and no route: a dependency tool
//! template is meant to travel — handed from one caller to another,
//! hashed the same everywhere — and a name is one daemon's word for
//! one thing, while a tag is a convention a template may assume. There
//! is no `routes` kind, since a route is a position named by an agent
//! and a path. The database, of which there is one, is a bare array of
//! actions, as it is for the daemon.
//!
//! # The actions are defined once
//!
//! Each kind's [`Make`](agents::Make) and [`Over`](agents::Over) — what
//! brings one into being, what is done to one that exists — live here,
//! and the daemon's grants are built of them:
//! [`daemon::grant`](crate::daemon::grant) re-exports every one, so a
//! grant of a template and a grant of a role hold the same actions and
//! differ only in how far they reach. [`Tagging`] and [`Within`] are
//! the daemon's too, defined here for the same reason.
//!
//! # Where it is carried
//!
//! A [`Template`](crate::shared::containers::dependencies::Template)
//! carries a list of [`Grant`]s: what the account its tool runs under
//! holds. The provider relays them and reads nothing; the daemon, when
//! it makes the tool, gives it an account holding exactly these and
//! judges everything the tool asks of it by them, as
//! [`daemon::grant`](crate::daemon::grant) states a request is judged.
//! Every type here derives a JSON Schema, because an image embeds the
//! template in the schema of its own arguments.

mod grant;
mod tags;
mod tagging;
mod within;

pub use grant::*;
pub use tags::*;
pub use tagging::*;
pub use within::*;

pub mod accounts;
pub mod agents;
pub mod agents_templates;
pub mod postgres;
pub mod providers_incoming;
pub mod providers_outgoing;
pub mod roles;
pub mod tools;
pub mod tools_templates;
pub mod volumes;
