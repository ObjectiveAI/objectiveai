//! One FUSE mount added to a running container: its serve opened,
//! the provider asked and waited for, and the mount kept only when
//! the provider made it.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle;
use diverge_sdk::shared::containers::fuse::Kind;
use diverge_sdk::shared::containers::fuse_mount::response::Frame;

use crate::containers::ToolHandle;
use crate::containers::fuse::Mounts;
use crate::daemon::Daemon;

/// The run the mount goes into, by the handle its provider is asked
/// on.
pub enum Which<'a> {
    /// An agent's run: the scope held on its container.
    Agent(&'a ExecuteHandle),
    /// A tool's run.
    Tool(&'a ToolHandle),
}

/// How one added mount ended.
pub enum Outcome {
    /// The provider mounted it; it is served, announced, and to be
    /// put on record.
    Mounted,
    /// The provider answered, and did not mount it: its answer, for
    /// the `mount` error.
    Refused(Frame),
    /// The serve could not be opened, or the provider was not heard
    /// from: the daemon's own words.
    Failed(String),
}

/// Mount `added` into the running container, and say how it ended.
/// The rules, in order:
///
/// 1. The mount's serve is opened on the volume's provider, as the
///    run's mounts were at its start — the provider connected, its
///    capabilities serving the volume's mode, the serve opened — and
///    the mount is kept under a fresh id, on the list its kind
///    names. What cannot be is `Failed`, and nothing was asked.
/// 2. The container's provider is asked, on the run's `fuse_mount`
///    channel, to mount it at its path under that id, with its
///    kind, and its one answer is waited for, however long the proxy
///    takes.
/// 3. `Mounted` is the mount made: it is announced to whoever watches
///    the container, and stays. Any other answer is the mount not
///    made: its serve is let go, and the answer is `Refused`. A
///    channel that ended unanswered, or an ask that could not be
///    sent, is the mount not made and `Failed`.
pub async fn runtime_mount(daemon: &Daemon, mounts: &Arc<Mounts>, which: Which<'_>, added: &FuseMount, directory: bool) -> Outcome {
    let fuse = match mounts.add(daemon, added, directory).await {
        Ok(fuse) => fuse,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    let kind = if directory { Kind::Directory } else { Kind::File };
    let asked = match which {
        Which::Agent(handle) => handle
            .fuse_mount(fuse.container_path.clone(), fuse.id.clone(), kind)
            .await
            .map_err(|error| error.to_string()),
        Which::Tool(handle) => handle.fuse_mount(fuse.container_path.clone(), fuse.id.clone(), kind).await,
    };
    match asked {
        Ok(Frame::Mounted) => {
            mounts.announce(&fuse.id).await;
            Outcome::Mounted
        }
        Ok(frame) => {
            mounts.drop_mount(&fuse.id).await;
            Outcome::Refused(frame)
        }
        Err(error) => {
            mounts.drop_mount(&fuse.id).await;
            Outcome::Failed(error)
        }
    }
}
