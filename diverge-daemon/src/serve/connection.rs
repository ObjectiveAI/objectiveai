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

use super::{judge, refuse};

/// Serve `connection` from `address` until it ends.
///
/// The client dialled, so the client speaks first, and the first
/// thing it says must be who it is: a connection that ends before
/// that, or that opens a scope before it, is dropped as it is, with
/// nothing finished — a client that has not been admitted cannot make
/// this end compose a reply. The credential is [`judge`]d, and a
/// refusal is the connection closed without a word, as a provider
/// closes on a key it does not hold. Admitted, every scope the client
/// opens is read as a request and handed to [`refuse`] on a task of
/// its own, so a slow answer never holds the socket; a second
/// credential ends the connection. When the socket is gone, every
/// task still answering is waited for, since an answer composed is
/// an answer sent.
pub async fn connection(connection: Connection, address: IpAddr) {
    let mut session = Session::new(connection);
    let Some(Received::Auth(payload)) = session.next().await else {
        return;
    };
    let credential = match Auth::decode(&payload) {
        Ok(Auth::Unbrokered(credential)) => credential,
        Err(_) => return,
    };
    let Some(account) = judge(credential, address).await else {
        return;
    };
    let account: Arc<str> = account.into();
    let mut scopes = JoinSet::new();
    while let Some(received) = session.next().await {
        while scopes.try_join_next().is_some() {}
        let Received::Request(payload, scope) = received else {
            break;
        };
        let account = Arc::clone(&account);
        scopes.spawn(async move {
            refuse(scope, &payload, &account).await;
        });
    }
    while scopes.join_next().await.is_some() {}
}
