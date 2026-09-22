//! Agents: what the daemon runs on a caller's behalf, and names.
//!
//! An agent is an agent container the daemon spawns on a provider,
//! from the same [`Container`](diverge_provider_sdk::shared::containers::request::Container)
//! request a caller would send that provider itself — the image, the
//! limits, the mounts, the arguments — and holds under a name of the
//! caller's choosing. The name is how the caller reaches the agent
//! after: one name, one agent, for as long as the agent exists.
//!
//! [`create`] spawns an agent under a name; [`delete`] removes one by
//! name; [`message`] sends one a message, and may take it back;
//! [`logs`] queries what one said and what was said to it.

pub mod create;
pub mod delete;
pub mod logs;
pub mod message;
