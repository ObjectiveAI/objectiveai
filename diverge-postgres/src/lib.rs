//! The local Postgres of a Diverge daemon, supervised.
//!
//! A daemon in its local mode serves its containers a Postgres it
//! runs itself. This program is that Postgres: the binaries baked into
//! it are extracted once into its directory, a cluster is initialized
//! there once, and every start finds the cluster, stops whatever
//! postmaster the last start left, starts a fresh one on a free
//! loopback port, announces the URL on stdout as one JSON line, and
//! waits — for a shutdown line on stdin, for Ctrl-C or SIGTERM, or for
//! the postmaster to exit on its own — then stops the postmaster
//! cleanly and exits.
//!
//! # No leash
//!
//! The postmaster is not tied to this program's life. A supervisor
//! that is killed, or that dies, leaves its postmaster running, and
//! the data safe; the next start finds that postmaster by the
//! cluster's own `postmaster.pid`, stops it through `pg_ctl`, and
//! starts its own. The postmaster carries the name `diverge-postgres`
//! as its `cluster_name`, which is what `ps` shows beside every
//! process of it, so a human can find and clean up what a start has
//! not yet. What this buys is that nothing a parent does by accident
//! takes the database down with it; what it costs is the one `pg_ctl
//! stop` at the next start.
//!
//! # The directory
//!
//! `--config <dir>`, else `DIVERGE_POSTGRES_CONFIG`, else
//! `~/.diverge/postgres/`. Inside it: `config.yaml`, optional; `bin/`,
//! where the binaries are extracted and where EVERY lock file lives,
//! under `bin/locks/`; `data/`, the cluster, with `data.ready` beside
//! it as the mark that its initialization finished; and `password`,
//! the superuser's, minted at the first start and read at every one.
//!
//! [`config`] is what the program is told; [`install`] the binaries;
//! [`cluster`] the password and the initialization; [`postmaster`]
//! the stop, the start and the readiness of the server process;
//! [`serve`] the whole, in order, and the stop. The two lines that
//! cross its stdio are the SDK's
//! [`postgres_supervisor`](diverge_sdk::postgres_supervisor), so the
//! daemon that starts it names them from the same place.

pub mod cluster;
pub mod config;
pub mod install;
pub mod postmaster;
pub mod serve;
