//! What lands as a resource, kept.

use diverge_sdk::daemon::creator::Creator;

use crate::content::{self, Received};
use crate::daemon::{Daemon, Kind};
use crate::store::resources;

/// Keep the received bytes as a resource with the description: held
/// anew, or found held already — the same id, and the description
/// now this one. The id.
pub async fn keep(daemon: &Daemon, received: Received, description: String, creator: Creator) -> Result<String, String> {
    let mut tx = daemon.store.begin().await.map_err(|error| error.to_string())?;
    resources::hold(
        &mut tx,
        &resources::New {
            id: received.id.clone(),
            kind: received.kind,
            description,
            bytes: received.bytes,
            creator,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    if let Err(error) = content::place(&daemon.resources, &received.incoming, &received.id).await {
        content::discard(&received.incoming).await;
        return Err(error.to_string());
    }
    tx.commit().await.map_err(|error| error.to_string())?;
    daemon.live.changed(Kind::Resources);
    Ok(received.id)
}
