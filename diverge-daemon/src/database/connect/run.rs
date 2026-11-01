//! A connection, from the dial taken to the end.

use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use diverge_sdk::daemon::agent_log::{self as log, Item};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::watch;

use super::{AUTHENTICATION_OK, Packet, auth, cancel, dial, error_response, read_startup, relay};
use crate::containers::{self, Key};
use crate::daemon::Daemon;
use crate::database::scope;

/// Serve one container connection: everything the container writes
/// arrives on `from_container`, everything the database says goes
/// out on `to_container`, and dropping the sender is the connection
/// closed. The scope made if it was not; the startup packet read; a
/// cancel sent on and done; else the database dialled and the
/// handshake performed as the container's role, the container
/// answered `AuthenticationOk` and what the database said, and the
/// rest relayed. A failure before the relay is an `ErrorResponse` to
/// the container, in the server's words when there are any, and the
/// end; for an agent, also an `error` item in its log.
pub async fn run(
    daemon: Arc<Daemon>,
    key: Key,
    mut from_container: Receiver<Bytes>,
    to_container: Sender<Bytes>,
    touched: watch::Sender<Instant>,
) {
    let scope = match scope(&daemon, key).await {
        Ok(scope) => scope,
        Err(why) => {
            let _ = to_container.send(error_response("08004", &why)).await;
            note(&daemon, key, &why).await;
            return;
        }
    };
    let Some(packet) = read_startup(&mut from_container, &to_container).await else {
        return;
    };
    let parameters = match packet {
        Packet::Startup(parameters) => parameters,
        Packet::Cancel { process_id, secret_key } => {
            cancel::cancel(&daemon, key, process_id, secret_key).await;
            return;
        }
    };
    let (mut server, encrypted) = match dial::dial(&daemon.database).await {
        Ok(dialled) => dialled,
        Err(why) => {
            let _ = to_container.send(error_response("08006", &why)).await;
            note(&daemon, key, &why).await;
            return;
        }
    };
    let established = match auth::handshake(&mut server, encrypted, &daemon.database, &scope, parameters).await {
        Ok(established) => established,
        Err(failure) => {
            let _ = to_container.send(match failure.response {
                Some(response) => response,
                None => error_response("08006", &failure.why),
            }).await;
            note(&daemon, key, &failure.why).await;
            return;
        }
    };
    if to_container.send(Bytes::from_static(&AUTHENTICATION_OK)).await.is_err() {
        return;
    }
    for message in established.replay {
        if to_container.send(message).await.is_err() {
            return;
        }
    }
    let backend = established.backend;
    if let Some(backend) = backend {
        daemon.live.register_backend(backend, key).await;
    }
    let opened = daemon.live.open_connection(key).await;
    touched.send_replace(Instant::now());
    relay::relay(server, from_container, to_container, touched).await;
    daemon.live.close_connection(opened).await;
    if let Some(backend) = backend {
        daemon.live.forget_backend(backend).await;
    }
}

/// Why the connection was refused, kept in an agent's log.
async fn note(daemon: &Daemon, key: Key, why: &str) {
    if let Key::Agent(id) = key {
        let _ = containers::append_for(
            daemon,
            id,
            Item::Error(log::Error {
                r#type: Default::default(),
                error: diverge_sdk::shared::error::Error(serde_json::json!({
                    "kind": "database",
                    "error": why,
                })),
            }),
        )
        .await;
    }
}
