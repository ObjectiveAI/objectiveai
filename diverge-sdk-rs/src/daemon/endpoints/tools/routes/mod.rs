//! Routes: a tool dependency's position, answered by a tool that
//! already exists.
//!
//! When a container registers, it declares the tool containers it
//! depends on — each a tool template with instructions, as the
//! provider protocol carries them — and somebody has to make each one
//! real. A ROUTE says that at one POSITION the daemon serves a tool
//! the caller already has and asks nobody. The position is a
//! [`Path`]: the name of the agent whose run began the chain, and
//! the template ids down the chain of dependencies, the last being
//! the dependency's own template. When a run reaches a position and
//! a route is there, the routed tool is attached and served, and the
//! container's `deployer_agent` is not asked; when none is there, the
//! deployer is handed the dependency, makes a tool, and may [`set`] a
//! route so that the next run at that position is answered without
//! it. A container with no deployer and no route for a dependency has
//! that dependency unmet.
//!
//! A path has at most one route. Only a tool made from the path's
//! last template may be routed there: the dependency IS that
//! template, and a tool of another is not the dependency, whatever
//! its name. A connected tool, made from no template of the caller's,
//! is routed nowhere. [`set`] puts a route down — once: a position
//! routed already is not routed again until its route is deleted —
//! [`delete`] takes
//! one up — refused while an active container is served through it
//! — and [`list`] names them, narrowed, each with the tool it routes
//! to.

mod path;

pub use path::*;

pub mod delete;
pub mod list;
pub mod set;
