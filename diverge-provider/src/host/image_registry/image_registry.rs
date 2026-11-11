//! The registry: started on its port, minting and revoking tokens.

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::{Error, Origin, Registry, Repositories, Repository, router};
use crate::host::image_manager::Store;

/// The provider's registry: an HTTP server on an ephemeral loopback
/// port, serving one repository per token minted.
///
/// Started once by [`start`](Self::start), which binds the port and
/// spawns the server on the runtime; [`address`](Self::address) is
/// then what the deployer points the runtime at. Dropping it aborts
/// the server — the provider's shutdown, when the runs are over.
#[derive(Debug)]
pub struct ImageRegistry {
    address: SocketAddr,
    repositories: Repositories,
    server: JoinHandle<()>,
}

impl ImageRegistry {
    /// Bind a loopback port the OS picks, and serve on it from now
    /// until this is dropped, every token from `store`.
    pub async fn start(store: Arc<Store>) -> Result<Self, Error> {
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

    /// A token minted for one pull, served from `origin` until it is
    /// [revoked](Self::revoke): 16 bytes from the operating system's
    /// cryptographic source as 32 lowercase hexadecimal characters
    /// (see [`image_registry`](super), rule 1). A source that cannot
    /// answer is [`Error::Random`]; a token minted twice — which 128
    /// random bits never are — is minted again.
    pub fn mint(&self, origin: Origin) -> Result<String, Error> {
        let repository = Arc::new(Repository::new(origin));
        loop {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(|error| Error::Random(error.to_string()))?;
            let token = hex::encode(bytes);
            if let dashmap::Entry::Vacant(vacant) = self.repositories.entry(token.clone()) {
                vacant.insert(Arc::clone(&repository));
                return Ok(token);
            }
        }
    }

    /// The token revoked: every path under it is `404` from now on. A
    /// response in flight keeps its own handle and finishes. A token
    /// not minted is nothing to do.
    pub fn revoke(&self, token: &str) {
        self.repositories.remove(token);
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
