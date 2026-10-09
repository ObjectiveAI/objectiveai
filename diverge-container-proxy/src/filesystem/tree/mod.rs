//! The filetree: the container's filesystem, or a subtree of it,
//! watched and streamed.

mod deltas;
mod ignore;
mod stream;
mod walk;
mod watch;

pub use deltas::*;
pub use ignore::*;
pub use stream::*;
pub use walk::*;
pub use watch::*;
