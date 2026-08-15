//! What every request for a container has to say, whatever kind it is.
//!
//! The rest of [`container`](super) is about a container that already
//! exists — reading from one, writing into one, moving a file between
//! two. This is the part that comes first: asking for one at all.
//!
//! [`Image`] is what lives here so far, and it is here because every
//! kind of container is made from an image and the question of who
//! supplies it has the same three answers each time. A
//! [`laboratory`](crate::endpoints::laboratories::run) and an
//! [`mcp_plugin`](crate::endpoints::mcp_plugin) differ in nearly
//! everything else about how they are run and not at all in this.

mod image;

pub use image::*;
