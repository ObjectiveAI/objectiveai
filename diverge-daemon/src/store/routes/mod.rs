//! The route records: at one position — an agent, and a dependency's template — the
//! tool that answers.
//!
//! [`Route`] is one as the store holds it: the position — the agent
//! that asks, by name, and the dependency's template — the tool, who put it down and when. A position has at most one
//! route, and the tool gone takes the route with it. [`all`],
//! [`by_path`] and [`of_tool`] load; [`create`] puts one down;
//! [`delete`] takes one up.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod load;
mod route;
mod row;

pub use create::*;
pub use delete::*;
pub use load::*;
pub use route::*;
