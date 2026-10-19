//! Grants over dependency templates.
//!
//! What a role may allow over dependency templates and the tools
//! running from them, in two of the shapes [`grant`](crate::daemon::grant)
//! states: [`Over`], the actions over those that exist — defined once
//! in [`shared::permission`](crate::shared::permission) and
//! re-exported — and [`Permission`], one grant's worth of them, or of
//! tagging. There is no making shape: a dependency template comes to
//! be by an agent's registration, and nothing on the wire makes one.

mod permission;

pub use crate::shared::permission::dependency_tools::Over;
pub use permission::*;
