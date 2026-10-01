//! Create request data.
//!
//! What a caller hands the daemon to create a tool under a name:
//! the [`Frame`] names the
//! [`template`](crate::daemon::endpoints::tools::templates) the tool
//! is made from and carries what is the tool's own: the one
//! [`Provider`] it is pinned to with the [`VolumeMount`]s it has
//! there, if any, its [`FuseMount`]s from whichever providers hold
//! them, and the name. The types are the agents create's, named here
//! as well: a tool container is made of what an agent container is
//! made of.

mod frame;

pub use frame::*;
pub use crate::daemon::endpoints::agents::create::client::request::{FuseMount, Provider, VolumeMount};
