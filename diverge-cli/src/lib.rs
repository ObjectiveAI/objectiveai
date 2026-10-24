//! The Diverge CLI: the daemon's client as a program.
//!
//! `diverge` dials one daemon as one account and puts the daemon's
//! requests on the command line. Which daemon and which account are
//! the document's, `cli.yaml` under the ROOT: the root is
//! `--config <dir>`, else `DIVERGE_CONFIG`, else `.diverge` in the
//! working directory where that directory exists — `cli.yaml` in it
//! or not — else `.diverge` under the home directory, as every
//! Diverge program finds its root but for the working directory's
//! turn; see [`config`]. The document names daemons by address, each
//! with named accounts — each an `(issuer, subject)`, one of the
//! root's OpenID Connect identities — and a default account, and a
//! default daemon; with no file it is the built-in default, the daemon
//! `local` on this host at the daemon's own default port, with no
//! account. No token is in the document: every identity is under the
//! root's `identities/`, which the command `identities` signs in —
//! the browser opened at the issuer — lists and removes, and no
//! command prints a token.
//!
//! [`arguments`] is the command line; [`cli`] runs it. Every request of
//! the daemon's wire is reached under the one command `run`, as a
//! subcommand under its family — `run agents create`, `run
//! standalone-tools templates list` — with the daemon and the account
//! by name, else the defaults, and the body as JSON, whose schema
//! `request-schema` beneath the request prints, and whose every answer
//! `response-schema` states; see [`run`]. Each program's document in
//! the root is read, rewritten and edited beneath the program —
//! `daemon config`, `provider config`, `issuer config`, `viewer config`,
//! `cli config` — through jq, as JSON on this side; see [`document`].
//! The commands `daemon start|stop`, `provider start|stop` and `issuer
//! start|stop` start the program on the root — its own
//! binary beside this one, spawned detached and waited for until it
//! holds the root's lock — and stop it, by one `POST /stop` on its
//! own port from this host, waited for until the lock is free; see
//! [`lifecycle`]. The issuer takes no commands of its own: its
//! identities and its signing keys are worked on here, in its
//! `issuer.sqlite` — `issuer identities add|list|remove|password`,
//! `issuer keys rotate [--sign-out]|list` — see [`cli::issuer`].

pub mod arguments;
pub mod cli;
pub mod config;
pub mod document;
pub mod jq;
pub mod lifecycle;
pub mod run;
