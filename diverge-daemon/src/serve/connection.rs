//! One accepted socket, for its life.

use std::net::IpAddr;
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use diverge_sdk::wire::server::received::Received;
use diverge_sdk::wire::server::session::Session;
use futures_util::StreamExt as _;
use tokio::task::JoinSet;

use super::dispatch;
use crate::daemon::Daemon;
use crate::store::AccountId;
use crate::judge::{Admitted, Expected, Who};
use crate::providers;

/// Serve `connection` from `address` until it ends.
///
/// Whoever dialled speaks first, and the first thing it says is who
/// it is: the [`handshake`](diverge_sdk::wire::handshake), made here
/// as the connectee, since a client and a provider dial the one port
/// and open the one way, and which half of the wire the daemon holds
/// toward the socket is decided by what the verified identity is.
/// The [`DaemonAuthenticator`](crate::judge::DaemonAuthenticator)
/// verifies the connector's token and refuses an identity that is
/// neither an account nor an accepted provider, and a refusal — or a
/// store that could not be asked — is the connection closed without a
/// word. A provider admitted is [`attach`](providers::attach)ed, where
/// the daemon is the caller — and where a second connection under an
/// identity a connection holds is dropped the same way; a client
/// admitted is served here, as a client is.
pub async fn connection(connection: Connection, address: IpAddr, daemon: Arc<Daemon>) {
    let Ok(Handshook { connection, peer }) = handshake::accept(connection, &daemon.authenticator.expecting(Expected::Admitted), address).await else {
        return;
    };
    match daemon.authenticator.admit(&peer).await {
        Ok(Some(Admitted::Client(who))) => client(connection, who, daemon).await,
        Ok(Some(Admitted::Provider(identity))) => {
            if let Ok(attached) = providers::attach(connection, identity, &daemon).await {
                attached.serve().await;
            }
        }
        Ok(None) | Err(_) => {}
    }
}

/// Serve a client admitted as `who`, past its handshake, until the
/// socket ends.
///
/// The connection counts itself in as its account — a tool's, served
/// under the grants it declared, is no account's — and every
/// scope the client opens is read as a request and handed to
/// [`dispatch`] on a task of its own, so a slow answer never holds
/// the socket; a frame that is not a request ends the connection. When the
/// socket is gone it is dropped first, which tells every scope still
/// being served that its feed has closed, and then every task still
/// answering is waited for, since an answer composed is an answer
/// sent, and the connection counts itself out.
pub async fn client(connection: Connection, who: Who, daemon: Arc<Daemon>) {
    let mut session = Session::new(connection);
    match &who {
        Who::Account(account) => {
            if daemon.live.enter(*account).await {
                connected_at(&daemon, *account).await;
            }
        }
        Who::Admin(identity) => daemon.live.enter_admin(identity).await,
        Who::Tool(_) => {}
    }
    let mut scopes = JoinSet::new();
    while let Some(received) = session.next().await {
        while scopes.try_join_next().is_some() {}
        let Received::Request(payload, scope) = received else {
            break;
        };
        let daemon = Arc::clone(&daemon);
        let who = who.clone();
        scopes.spawn(async move {
            dispatch(scope, &payload, who, &daemon).await;
        });
    }
    drop(session);
    while scopes.join_next().await.is_some() {}
    match &who {
        Who::Account(account) => {
            if daemon.live.leave(*account).await {
                connected_at(&daemon, *account).await;
            }
        }
        Who::Admin(identity) => daemon.live.leave_admin(identity).await,
        Who::Tool(_) => {}
    }
}

/// Keep now as the account's `last_connected`: its first connection
/// began, or its last ended. A time that cannot be kept fails nothing.
async fn connected_at(daemon: &Daemon, account: AccountId) {
    if let Ok(mut conn) = daemon.store.acquire().await {
        let _ = crate::store::accounts::set_connected(&mut conn, account, chrono::Utc::now()).await;
    }
}
