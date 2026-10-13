//! Dependency templates: what an agent's program declares, held by
//! its hash, with the tools running from it.
//!
//! A dependency template is a
//! [`Template`](crate::shared::containers::dependencies::Template) an
//! agent's program declared at its registration — transient, deployed
//! for the agent's run, or a service, kept under the scope its
//! `scope` names — made a record of the daemon's by the first
//! registration that declares it and held by its id, the hash of its
//! canonical bytes, so the same template declared twice is one.
//! Nothing on this wire makes one: a registration does, and every
//! registration RESETS what the agent depends on — exactly the
//! templates it declared then are its dependencies, and one declared
//! before and not again is its dependency no more. A template outlives
//! the agents that declared it, and so do the database scopes the
//! tools running from it were given, under every agent, agent
//! template and the global one, until the template is deleted:
//! [`delete`] removes one no tool runs from — transient or service —
//! with every scope ever made from it, while agents still depending on
//! it depend on it still and deploy it anew, as a new record, at their
//! next registration; [`get`] answers one by id; [`list`] lists them,
//! narrowed, each with the agents whose last registration declared it
//! and the tools running from it now; [`tag`] and [`untag`] change a
//! template's tags, which are the caller's and not in its hash.
//!
//! # The tools
//!
//! A tool running from a dependency template is no record: it is
//! listed under its template, by whose it is and where it runs, and
//! nowhere else — never among the [`tools`](super::tools), whose
//! endpoints find no dependency tool. It is named by its template and
//! its owner, a
//! [`reference::Dependency`](crate::daemon::reference::Dependency),
//! for the four operations on a running container's files:
//! [`filetree`] watches one whole; [`download`] sends the client a
//! file or a directory out of it, [`upload`] puts files into it, and
//! [`transfer`] copies out of it into an agent, a tool, a dependency
//! tool or a volume, the bytes never reaching the client. Each finds
//! the tool running or answers `NotFound`, and starts and stops
//! nothing.

pub mod delete;
pub mod download;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;
