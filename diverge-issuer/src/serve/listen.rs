//! The issuer's port.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::{Error, Issuer, stop_route};
use crate::endpoints;

/// Listen on `port`, on every interface, until `stop` says so. A
/// request at a path the issuer does not serve is answered as axum
/// answers one. When `stop` turns `true` the listener stops
/// accepting, closes what is idle and waits for the requests in
/// flight.
pub async fn listen(issuer: Arc<Issuer>, port: u16, mut stop: watch::Receiver<bool>) -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.map_err(Error::Bind)?;
    let router = Router::new()
        .route(endpoints::DISCOVERY, get(endpoints::discovery))
        .route(endpoints::JWKS, get(endpoints::jwks))
        .route(endpoints::AUTHORIZE, get(endpoints::authorize_page).post(endpoints::authorize_decide))
        .route(endpoints::DEVICE_AUTHORIZATION, post(endpoints::device_authorization))
        .route(endpoints::DEVICE, get(endpoints::device_page).post(endpoints::device_decide))
        .route(endpoints::TOKEN, post(endpoints::token))
        .route(endpoints::REVOKE, post(endpoints::revoke))
        .route(endpoints::USERINFO, get(endpoints::userinfo).post(endpoints::userinfo))
        .route("/stop", post(stop_route::stop))
        .with_state(issuer);
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        })
        .await
        .map_err(Error::Serve)
}
