//! Agents: what the daemon runs on a caller's behalf, and names.
//!
//! An agent is an agent container the daemon spawns on a provider —
//! one the daemon chooses, or the one its create pins it to, with
//! that provider's volumes mounted — from what the [`create`] names:
//! a [`template`](templates), which is the image, the limits, the
//! resources served over FUSE and the arguments, held by its hash —
//! the daemon's one [`template`](crate::daemon::template) shape, typed
//! `"agent"` —
//! and what is the agent's own: the provider it runs on, and the
//! mounts of providers' volumes. The daemon holds it
//! under a name of the caller's choosing. The name is how the caller reaches the agent
//! after: one name, one agent, for as long as the agent exists.
//!
//! [`templates`] holds what agents are made from; [`create`] spawns
//! an agent under a name from one; [`delete`] removes one by
//! name; [`edit`] changes what one mounts; [`message`] sends one a message, and may take it back;
//! [`logs`] reads what one said and what was said to it; [`list`]
//! names every one the caller has, with its tags; [`tag`] and
//! [`untag`] change one's tags. The MCP servers an agent calls are
//! [`tools`](super::tools), attached to it by name.

pub mod create;
pub mod delete;
pub mod edit;
pub mod list;
pub mod logs;
pub mod message;
pub mod tag;
pub mod templates;
pub mod untag;
