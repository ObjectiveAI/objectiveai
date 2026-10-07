//! A container's scope as it is live: the password in memory, and the
//! scope made once per daemon life.

use std::sync::Arc;

use rand::RngCore as _;
use tokio::sync::OnceCell;

use super::{container_of, provision, role_of};
use crate::containers::Key;
use crate::daemon::Daemon;

/// One container's scope, live: its role, the password the daemon
/// minted for this daemon's life, and whether the scope has been made
/// — or found made and given this password — since the start.
#[derive(Debug)]
pub struct Scope {
    /// The role, and the schema.
    pub role: String,
    /// The password, hex of thirty-two random bytes; in memory only.
    pub password: String,
    ready: OnceCell<()>,
}

impl Scope {
    /// A scope not yet made this daemon life, with a fresh password.
    pub fn new(role: String) -> Scope {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Scope {
            role,
            password: hex::encode(bytes),
            ready: OnceCell::new(),
        }
    }
}

/// The container's scope, made if it was not this daemon life: the
/// identity looked up, the daemon's role checked able, the four
/// statements run once — a pool opening many connections at once
/// waits on the first. The failure, in a sentence: a record gone, a
/// role that cannot provision, the store.
pub async fn scope(daemon: &Daemon, key: Key) -> Result<Arc<Scope>, String> {
    let container = {
        let mut conn = daemon.store.acquire().await.map_err(|error| error.to_string())?;
        container_of(&mut conn, key)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "the container's record is gone".to_string())?
    };
    let scope = daemon.live.scope(key, || role_of(&container)).await;
    scope
        .ready
        .get_or_try_init(|| async {
            let mut tx = daemon.store.begin().await.map_err(|error| error.to_string())?;
            provision::capable(&mut tx).await.map_err(|error| error.to_string())??;
            provision::ensure(&mut tx, &scope.role, &scope.password)
                .await
                .map_err(|error| error.to_string())?;
            tx.commit().await.map_err(|error| error.to_string())?;
            Ok::<(), String>(())
        })
        .await?;
    Ok(scope)
}
