//! A directory of this host, walked and watched: the filetree of a
//! volume the provider watches itself.
//!
//! One module, every host. The three platforms — inotify on Linux,
//! FSEvents on macOS, `ReadDirectoryChangesW` on Windows — are
//! `notify`'s recommended watcher, one API over all three, and what
//! this module adds is platform-neutral: the root made canonical once,
//! so the paths the watcher reports strip against it whatever the
//! host spells them as (`\\?\` on Windows, `/private` on macOS); a
//! registration that degrades per directory where inotify's watch
//! limit bites, with the directories it could not watch reported
//! `changes` false; and the walk and every event's mapping run under
//! `spawn_blocking`, one event at a time so the frames' order is the
//! events' order.
//!
//! The rules the container proxy's tree keeps are kept here. The
//! watcher is armed BEFORE the walk, so a change during the walk
//! arrives as a delta after the snapshot rather than falling between
//! the two. Lost events — the queue overflowed, notify reported an
//! error — are not the watch's error: the tree is walked again and a
//! fresh snapshot sent. Nothing is debounced: one event, its frames.
//! What IS the watch's error is a root that is not a directory, a
//! watcher that could not be made or could not watch the root, a
//! blocking task that died, or a watcher that stopped on its own; the
//! stream yields the error once and ends.
//!
//! [`watch`] opens one, [`Watch`] is the stream, [`Error`] what it
//! fails with. `paths`, `walk`, `register` and `deltas` are the
//! steps: the root and the components, the tree read from disk, the
//! watcher armed and registered, and one event as the frames it means.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod deltas;
mod error;
mod paths;
mod register;
mod stream;
mod walk;

pub use deltas::*;
pub use error::*;
pub use paths::*;
pub use register::*;
pub use stream::*;
pub use walk::*;
