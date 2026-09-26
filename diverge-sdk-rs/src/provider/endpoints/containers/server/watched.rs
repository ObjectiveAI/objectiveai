//! A mount the provider watches on its own account, for the tree.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::{Stream, TryStreamExt as _};

use crate::provider::server::volume::Volume;
use crate::shared::error::Error;
use crate::shared::filetree::response::Frame;

/// A watch as the filetree relay reads it: the volume's frames, or
/// the error that ended it, rendered as the wire's.
pub(crate) type Watch = Pin<Box<dyn Stream<Item = Result<Frame, Error>> + Send>>;

/// One volume mount the container's tree leaves out, and the way to
/// watch it instead: opened once per filetree channel, and its frames
/// re-rooted at [`container_path`](Self::container_path) into the
/// tree the caller sees.
pub(crate) struct Watched {
    /// Where the mount is in the container: what every path the watch
    /// reports is prefixed with.
    pub container_path: Vec<String>,
    /// The watch, ready to open.
    pub watcher: Arc<dyn Watcher>,
}

impl fmt::Debug for Watched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Watched")
            .field("container_path", &self.container_path)
            .finish_non_exhaustive()
    }
}

/// Something that opens a watch, with the provider's types folded
/// away.
///
/// Boxed on purpose: a [`Run`](super::run::Run) is shared by every
/// task of a run and is not generic over the provider, and a
/// connector's run is built from what the directory holds, which is
/// not generic either. A watch is opened once per filetree channel,
/// so one dynamic call and one boxed stream per open cost nothing
/// that matters.
pub(crate) trait Watcher: Send + Sync {
    /// The watch, opened: the stream, or the error.
    fn watch(&self) -> Pin<Box<dyn Future<Output = Result<Watch, Error>> + Send + '_>>;
}

/// A volume and the path in it a mount descends to, as a [`Watcher`].
pub(crate) struct Watching<V: Volume> {
    /// The volume, held shared by the run for as long as this lives.
    pub volume: Arc<V>,
    /// The mount's `volume_relative_path`: what the watch is asked for.
    pub path: Vec<String>,
}

impl<V> Watcher for Watching<V>
where
    V: Volume + 'static,
    V::Error: Into<Error>,
{
    fn watch(&self) -> Pin<Box<dyn Future<Output = Result<Watch, Error>> + Send + '_>> {
        Box::pin(async move {
            let watch = self.volume.watch(&self.path).await.map_err(Into::into)?;
            Ok(Box::pin(watch.map_err(Into::into)) as Watch)
        })
    }
}
