//! An agent's declared dependencies, answered.
//!
//! An agent's program answers its registration with the tools it
//! depends on, each a tool template on record with instructions, and
//! the provider asks the daemon to deploy them before the container's
//! id is out. For each, [`template_of`] is the template the dependency
//! is, and [`position`] its place: the agent, by name, and that
//! template; [`deploy`] answers it — by a route at the position, by a
//! tool of that template attached to the agent, or by the deployer
//! agent, asked once in its queue and waited on until the position is
//! answered or the deployer goes inactive first. Answered
//! dependencies' containers are started as users of the agent being
//! deployed for and served to it under the declared name. A tool
//! container declares no dependencies and is never deployed for.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod deploy;
mod position;

pub use deploy::*;
pub use position::*;
