//! One accepted socket, for its life.

use std::net::IpAddr;
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::auth::Auth;
use diverge_sdk::wire::server::received::Received;
use diverge_sdk::wire::server::session::Session;
use futures_util::StreamExt as _;
use tokio::task::JoinSet;

use super::dispatch;
use crate::daemon::Daemon;
use crate::judge;

/// Serve `connection` from `address` until it ends.
///
/// The client dialled, so the client speaks first, and the first
/// thing it says must be who it is: a connection that ends before
/// that, or that opens a scope before it, is dropped as it is, with
/// nothing finished — a client that has not been admitted cannot make
/// this end compose a reply. The credential is
/// [`admit`](judge::admit)ted, and a refusal — or a store that could
/// not be asked — is the connection closed without a word, as a
/// provider closes on a key it does not hold. Admitted, the connection
/// counts itself in as its account, and every scope the client
/// opens is read as a request and handed to [`dispatch`] on a task of
/// its own, so a slow answer never holds the socket; a second
/// credential ends the connection. When the socket is gone it is
/// dropped first, which tells every scope still being served that its
/// feed has closed, and then every task still answering is waited
/// for, since an answer composed is an answer sent, and the connection
/// counts itself out.
pub async fn connection(connection: Connection, address: IpAddr, daemon: Arc<Daemon>) {
    let mut session = Session::new(connection);
    let Some(Received::Auth(payload)) = session.next().await else {
        return;
    };
    let credential = match Auth::decode(&payload) {
        Ok(Auth::Unbrokered(credential)) => credential,
        Err(_) => return,
    };
    let Ok(Some(who)) = judge::admit(&daemon.store, credential, address).await else {
        return;
    };
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
