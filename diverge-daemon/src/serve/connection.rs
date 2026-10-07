//! One accepted socket, for its life.

use std::net::IpAddr;
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::auth::Auth;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::received::Received;
use diverge_sdk::wire::server::session::Session;
use futures_util::StreamExt as _;
use tokio::task::JoinSet;

use super::dispatch;
use crate::daemon::Daemon;
use crate::judge::{self, Peer, Who};
use crate::providers;

/// Serve `connection` from `address` until it ends.
///
/// Whoever dialled speaks first, and the first thing it says must be
/// who it is: one frame, read here, since a client and a provider
/// dial the one port and present the one kind of frame, and which
/// half of the wire the daemon holds toward the socket is decided by
/// what the credential admits. A connection that ends before that, or
/// whose first frame is not a credential, is dropped as it is, with
/// nothing sent — a peer that has not been admitted cannot make this
/// end compose a reply. The credential is [`admit_peer`](judge::admit_peer)ed,
/// and a refusal — or a store that could not be asked — is the
/// connection closed without a word, as a provider closes on a key it
/// does not hold. A provider admitted is handed to
/// [`providers::incoming`], where the daemon is the caller; a client
/// admitted is served here, as a client is.
pub async fn connection(mut connection: Connection, address: IpAddr, daemon: Arc<Daemon>) {
    let Some(Ok(first)) = connection.next().await else {
        return;
    };
    let credential = match ClientFrame::decode(&first) {
        Ok(ClientFrame::Auth { payload }) => match Auth::decode(payload) {
            Ok(Auth::Unbrokered(credential)) => credential.to_string(),
            Err(_) => return,
        },
        _ => return,
    };
    let Ok(Some(peer)) = judge::admit_peer(&daemon.store, &credential, address).await else {
        return;
    };
    match peer {
        Peer::Client(who) => client(connection, who, daemon).await,
        Peer::Provider(identity) => {
            let _ = providers::incoming(connection, identity, &daemon).await;
        }
    }
}

/// Serve a client admitted as `who`, past its handshake, until the
/// socket ends.
///
/// The connection counts itself in as its account, and every scope
/// the client opens is read as a request and handed to [`dispatch`]
/// on a task of its own, so a slow answer never holds the socket; a
/// second credential ends the connection. When the socket is gone it
/// is dropped first, which tells every scope still being served that
/// its feed has closed, and then every task still answering is waited
/// for, since an answer composed is an answer sent, and the connection
/// counts itself out.
pub async fn client(connection: Connection, who: Who, daemon: Arc<Daemon>) {
    let mut session = Session::new(connection);
    daemon.live.enter(who.id).await;
    let mut scopes = JoinSet::new();
    while let Some(received) = session.next().await {
        while scopes.try_join_next().is_some() {}
        let Received::Request(payload, scope) = received else {
            break;
        };
        let daemon = Arc::clone(&daemon);
        scopes.spawn(async move {
            dispatch(scope, &payload, who, &daemon).await;
        });
    }
    drop(session);
    while scopes.join_next().await.is_some() {}
    daemon.live.leave(who.id).await;
}
