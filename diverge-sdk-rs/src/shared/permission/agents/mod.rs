//! Grants over agents.
//!
//! What a tool's program may ask over agents, in the
//! shapes [`permission`](crate::shared::permission) states: [`Make`],
//! the actions that bring one into being; [`Over`], the actions over
//! those that exist, reaching as far as [`Within`] says — every
//! agent, the tool's attached agents, those never active, the agents
//! listed by [`Key`], or by tags; and [`Permission`], one
//! grant's worth of either, or of tagging. The daemon's grants over
//! agents are made of the same [`Make`] and [`Over`], and reach by a
//! filter: the word `attached` and the list of agents are this kind's,
//! here, and nowhere else.

mod key;
mod make;
mod over;
mod permission;
mod within;

pub use key::*;
pub use make::*;
pub use over::*;
pub use permission::*;
pub use within::*;
