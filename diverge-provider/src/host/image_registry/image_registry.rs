//! The registry: started on its port, serving what the protocol asks
//! and what the store holds.

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use crate::protocol::image_registry;
use crate::protocol::image_source::ImageSource;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::{Error, Registry, Repositories, Repository, router};
use crate::host::image_manager::Store;

/// The provider's registry: an HTTP server on an ephemeral loopback
/// port, serving one repository per run and one per identity's pool.
///
/// Started once by [`start`](Self::start), which binds the port and
/// spawns the server on the runtime; [`address`](Self::address) is
/// then what the deployer points the runtime at. Dropping it aborts
/// the server, closing the listener and every connection on it —
/// the provider's shutdown, when the runs are over.
#[derive(Debug)]
pub struct ImageRegistry {
    address: SocketAddr,
    repositories: Repositories,
    server: JoinHandle<()>,
}

impl ImageRegistry {
    /// Bind a loopback port the OS picks, and serve on it from now
    /// until this is dropped: the run repositories as the protocol
    /// enters them, and the pools of `store`, if there is one.
    pub async fn start(store: Option<Arc<Store>>) -> Result<Self, Error> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(Error::Bind)?;
        let address = listener.local_addr().map_err(Error::Bind)?;
        let repositories: Repositories = Arc::new(DashMap::new());
        let registry = Registry {
            repositories: Arc::clone(&repositories),
            store,
        };
        let server = tokio::spawn(serve(listener, registry));
        Ok(ImageRegistry {
            address,
            repositories,
            server,
        })
    }

    /// Where it listens.
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

/// The server, for the listener's life. An error from it is the
/// listener gone, and there is nothing to do about that here.
async fn serve(listener: TcpListener, registry: Registry) {
    let _ = axum::serve(listener, router(registry)).await;
}

impl Drop for ImageRegistry {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl image_registry::ImageRegistry for ImageRegistry {
    type Error = Error;

    fn address(&self) -> SocketAddr {
        self.address
    }

    /// The repository entered, from now until released. A name
    /// already served is refused.
    async fn serve(&self, repository: &str, source: ImageSource) -> Result<(), Error> {
        match self.repositories.entry(repository.to_string()) {
            dashmap::Entry::Occupied(_) => Err(Error::Taken(repository.to_string())),
            dashmap::Entry::Vacant(vacant) => {
                vacant.insert(Arc::new(Repository::new(source)));
                Ok(())
            }
        }
    }

    /// The repository taken out; a name not served is nothing to do.
    /// A response in flight on it keeps its own handle to the
    /// repository and finishes.
    async fn release(&self, repository: &str) {
        self.repositories.remove(repository);
    }
}
