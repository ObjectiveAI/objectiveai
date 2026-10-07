//! What the file operations of every family share: a container
//! opened and closed for one, the chunks of a download, the channels
//! of an upload, a watch of a container's tree, a transfer's
//! destination judged, and the one cancel.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cancel;
mod destination;
mod download;
mod open;
mod upload;
mod watch;

pub use cancel::*;
pub use destination::*;
pub use download::*;
pub use open::*;
pub use upload::*;
pub use watch::*;
