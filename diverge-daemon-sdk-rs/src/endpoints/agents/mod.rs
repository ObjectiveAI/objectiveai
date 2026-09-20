//! Agents: what the daemon runs on a caller's behalf, and names.
//!
//! An agent is an agent container the daemon spawns on a provider,
//! from the same [`Container`](diverge_provider_sdk::shared::containers::request::Container)
//! request a caller would send that provider itself — the image, the
//! limits, the mounts, the arguments — and holds under a name of the
//! caller's choosing, its tag. The tag is how the caller reaches the
//! agent after: one tag, one agent, for as long as the agent lives.
//!
//! [`tag`] is the one scope so far: spawn an agent under a tag.

pub mod tag;
