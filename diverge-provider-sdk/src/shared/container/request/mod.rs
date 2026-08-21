//! What every request for a container has to say, whatever kind it is.
//!
//! The rest of [`container`](super) is about a container that already
//! exists — reading from one, writing into one, moving a file between
//! two. This is the part that comes first: asking for one at all.
//!
//! [`Image`] is here because every kind of container is made from one
//! and the question of who supplies it has the same three answers each
//! time. A [`laboratory`](crate::endpoints::laboratories::run) and an
//! [`mcp_plugin`](crate::endpoints::mcp_plugin) differ in nearly
//! everything else about how they are run and not at all in this.
//!
//! [`Mount`] is here for a sharper reason. It was a laboratory's, and
//! only a laboratory takes one today — but what puts a volume inside a
//! container is a fact about containers, and
//! [`ContainerDeployer`](crate::server::container_deployer::ContainerDeployer)
//! serves all three kinds. A generic deployer naming one endpoint's
//! type would have been the thing this crate spends its exceptions
//! avoiding.

mod image;
mod mount;

pub use image::*;
pub use mount::*;
