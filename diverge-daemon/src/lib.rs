//! The Diverge daemon: the server of the daemon protocol.
//!
//! A daemon is what a client of the daemon protocol —
//! [`diverge_sdk::daemon`] — asks for agents, tools, resources,
//! providers, accounts, roles, volumes and the database. The SDK
//! defines every request and every answer, and the frame-level
//! server, [`diverge_sdk::wire::server`], that reads scopes off a
//! socket; what it does not hold is the daemon itself — the judgment
//! of who is asking, the records of what exists, and the doing — and
//! this crate is where that goes. `ARCHITECTURE_1.md` in `reports/`
//! is the whole of it in one page; this is what of it exists.
//!
//! # What serves today
//!
//! Accounts, roles, providers, templates, resources, agents, tools
//! and routes, and the containers agents and tools are: the seventy
//! requests over them — the fourteen of the first two, the ten over
//! outgoing providers and incoming credentials, the listing of a
//! tenant's tool containers through a provider, the twelve over the
//! two template families, the nine over resources, the nine over
//! agents, their logs and their messages, the twelve over tools,
//! attachments and admissions, and the three over routes — are served
//! as the wire states them; every other request — the volumes, the
//! file movement into and out of containers, the database — is
//! answered with its endpoint's own error saying it is not served
//! yet; and every connection is judged. An agent's container runs on
//! a provider from its first message and stops after `idle_seconds`
//! unused; a tool's runs while an agent it is attached to is active;
//! the provider's asks on a run — the container's `/daemon`
//! connections, its tool calls, its mounts, its dependencies, who may
//! see or join it — are answered by [`containers`]. An outgoing provider on record is dialled and dialled
//! again for the daemon's life; a provider that dials in is admitted
//! by a credential the daemon minted; on either connection the daemon
//! is the caller of the provider protocol, [`providers`]. The records live in the one Postgres
//! the daemon runs on — its own, started beside it, or a remote one,
//! as [`config`] says — in a schema of the daemon's, [`store`]; who a
//! connection is and what each request may do is [`judge`]; what is
//! live and shared is [`daemon`]; the local Postgres is [`postgres`];
//! the connections to providers are [`providers`]; the bytes of every
//! resource are [`content`]'s, under `<dir>/resources/`; every agent's
//! log is [`logs`]', under `<dir>/agents/`; what runs, and
//! everything that runs through it, is [`containers`];
//! and [`serve`] is the daemon running: the port bound, every
//! connection read, every request dispatched, and the stop.
//!
//! # The first connection
//!
//! A fresh database — one with no accounts table before this daemon
//! initialized it — is seeded with a role `root` holding every grant
//! and an account `root` holding it, whose key is the word `root`.
//! Both are ordinary records: the first client connects with `root`,
//! makes the roles and accounts that are wanted, and rotates or
//! deletes root's credential with an edit, since until then the
//! daemon is open to whoever reaches its port. A database that has
//! the table is never seeded again, so a root deleted stays deleted.
//!
//! # The directory
//!
//! `--config <dir>`, else `DIVERGE_DAEMON_CONFIG`, else
//! `~/.diverge/daemon/`, holding `config.yaml`, optional, and
//! `postgres/`, the local cluster's own directory when the daemon
//! runs one, `resources/`, every resource's bytes by its hash,
//! `agents/`, every agent's log by its id, and `overlays/`, an
//! ephemeral mount's own layer for the run's life.

pub mod config;
pub mod containers;
pub mod content;
pub mod daemon;
pub mod judge;
pub mod logs;
pub mod postgres;
pub mod providers;
pub mod serve;
pub mod store;
