//! Dependency tools: one per dependency template and owner, as far as
//! the template's scope shares it, with the agents that depend on it
//! and the tool running from it.
//!
//! A dependency template is a
//! [`Template`](crate::shared::containers::dependencies::Template) an
//! agent's program declares at its registration — transient, deployed
//! for the agent's run, or a service, kept under the scope its `scope`
//! names — held by its id, the hash of its canonical bytes, so the
//! same template declared twice is one. A DEPENDENCY TOOL is that
//! template under one OWNER, which the template's scope decides: a
//! transient template, and a service whose scope is `instance`, is one
//! tool per agent instance, owned by that agent; a service whose scope
//! is `template` is one tool per agent template, owned by it; a
//! service whose scope is `global` is one tool, owned by nobody.
//! Nothing on this wire makes one: the first registration that
//! declares the template under that owner does, and every registration
//! RESETS what the agent depends on — exactly the tools of the
//! templates it declared then, each noting when the agent last
//! declared it, and a tool it declared before and not again is its
//! dependency no more. A tool outlives the registrations: it holds
//! its database scope until it is deleted. An instance tool goes with
//! its agent's delete and a per-template tool with its agent
//! template's, scope and all; a global tool goes with nothing but the
//! template's [`delete`], which removes every tool of the template
//! under every owner and is refused `InUse` while any of them runs.
//! [`get`] answers one tool by its template and its owner, as a list
//! would report it; [`list`] lists them, narrowed, each with the
//! agents that depend on it — at most one for an instance tool, every
//! agent of the one agent template for a per-template one, every agent
//! for a global one — and the one run from it, if its container is
//! up; [`tag`] and [`untag`] change one tool's tags, which are the
//! tool's and not in the template's hash.
//!
//! # The run
//!
//! A tool has at most one run: a transient tool's container, while its
//! agent's container runs; a service's, while the daemon keeps it up.
//! It is listed on the tool, by where it runs, and nowhere else —
//! never among the [`standalone_tools`](super::standalone_tools), whose
//! endpoints find no dependency tool. A tool is named by its template
//! and its owner, a
//! [`reference::Dependency`](crate::daemon::reference::Dependency),
//! for the four operations on a running container's files:
//! [`filetree`] watches one whole; [`download`] sends the client a
//! file or a directory out of it, [`upload`] puts files into it, and
//! [`transfer`] copies out of it into an agent, a tool, a dependency
//! tool or a volume, the bytes never reaching the client. Each finds
//! the tool running or answers `NotFound`, and starts and stops
//! nothing. [`logs`](super::logs) reads what the daemon kept of every
//! call and read it routed into one, running or not.

pub mod delete;
pub mod download;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;
