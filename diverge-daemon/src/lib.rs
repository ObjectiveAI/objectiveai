//! The Diverge daemon: the server of the daemon protocol.
//!
//! A daemon is what a client of the daemon protocol —
//! [`diverge_sdk::daemon`] — asks for agents, tools, providers,
//! accounts, roles, volumes and the database. The SDK
//! defines every request and every answer, and the frame-level
//! server, [`diverge_sdk::wire::server`], that reads scopes off a
//! socket; what it does not hold is the daemon itself — the judgment
//! of who is asking, the records of what exists, and the doing — and
//! this crate is where that goes. `ARCHITECTURE_1.md` in `reports/`
//! is the whole of it in one page; this is what of it exists.
//!
//! # What serves today
//!
//! Every request of the wire, all eighty-nine: the fourteen over
//! accounts and roles, the twenty-one over outgoing providers,
//! incoming credentials and the daemons reached through them, the
//! twelve over the two template families,
//! the thirteen over agents — their logs, their messages, and the
//! files of their containers — the fifteen over tools, attachments
//! and exposures and their containers' files, the twelve over
//! volumes, and the two over the database —
//! are served as the wire states them, and every connection is
//! judged. A volume is its provider's, found by asking, judged by what
//! the provider says and which records mount it, and held for the
//! length of any file operation on it, [`volumes`]; a file or a
//! directory moves between containers and volumes on
//! the daemon's own connections, [`transfers`], and a watch of a
//! container's tree has the daemon's mounts spliced in. An agent's container runs on
//! a provider from its first message and stops after `idle_seconds`
//! unused — active being a loop running or a call in flight, and
//! nothing else; a tool's runs while a container of the daemon's uses
//! it or a connector is attached to it from outside, and not a moment
//! longer; an agent's declared dependencies are deployed the moment
//! its container asks, tools of its own for its container's life;
//! the provider's asks on a run — the container's `/daemon`
//! connections, its tool calls, its mounts, its dependencies, who may
//! join it — are answered by [`containers`]; its database
//! connections reach one scope of its own, the handshake the daemon's
//! and the rest relayed unread, by [`database`]. An outgoing provider on record is dialled and dialled
//! again for the daemon's life; a provider that dials in is admitted
//! by a credential the daemon minted; on either connection the daemon
//! is the caller of the provider protocol, [`providers`]. Another
//! daemon on record is reached through a provider both are connected
//! to, as a client of it, [`daemons`]; a daemon that reaches this one
//! comes the same way, admitted by a credential as any client is,
//! through every provider this daemon accepts on. The records live in the one Postgres
//! the daemon runs on — its own, started beside it, or a remote one,
//! as its block of the one `config.yaml` says,
//! [`diverge_sdk::config::daemon`] — in a schema of the daemon's,
//! [`store`]; who a
//! connection is and what each request may do is [`judge`]; what is
//! live and shared is [`daemon`]; the local Postgres is [`postgres`];
//! the connections to providers are [`providers`]; what every file
//! movement is made of is [`content`]; every agent's
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
//! The root the SDK finds — `--config <dir>`, else `DIVERGE_CONFIG`,
//! else `~/.diverge/` — holds the one `config.yaml`, and the daemon
//! keeps its state under `<root>/daemon/`, which is `<dir>` wherever
//! this crate says it: `postgres/`, the local cluster's own directory
//! when the daemon runs one, and `agents/`, every agent's log by its
//! id.

pub mod containers;
pub mod content;
pub mod daemon;
pub mod daemons;
pub mod database;
pub mod judge;
pub mod logs;
pub mod postgres;
pub mod providers;
pub mod serve;
pub mod store;
pub mod transfers;
pub mod volumes;
