//! Finding a source that can supply an image: a provider, or a remote
//! daemon.

use std::collections::HashSet;
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::endpoints::images::read::client::{execute as daemon_execute, request as daemon_request};
use diverge_sdk::provider::endpoints::images::read::client::{execute, request};
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt as _, StreamExt as _};

use crate::daemon::Daemon;
use crate::daemons;
use crate::store;
use crate::store::daemons::Record;

/// Whom an ask may reach.
pub enum Reach {
    /// Every connected provider but `except`, and every remote daemon
    /// recorded: a provider asking on a run — which has looked in its
    /// own Store and its own registries already, and is `except` — or
    /// the daemon asking for itself, `except` then `None`. A provider
    /// asking needs no grant.
    Full { except: Option<Identity> },
    /// Exactly these providers and these remote daemons' records: what a
    /// command's grants reach.
    Granted { providers: HashSet<Identity>, daemons: Vec<Record> },
}

/// Who answered held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    /// A connected provider, by its identity.
    Provider(Identity),
    /// A remote daemon, by the record it was reached through.
    Daemon(DaemonId),
}

/// The one scope that answered held: the source an image's manifests,
/// blobs and proof are taken through. A provider's read scope and a
/// remote daemon's share their channels byte for byte, so either is
/// held by the same handle.
#[derive(Debug)]
pub struct Source {
    pub holder: Holder,
    pub scope: execute::ExecuteHandle,
}

/// Every ask of who can supply an image, each answering its source,
/// held, or `None`: what [`asks`] makes, taken as each answers.
pub type Asks = FuturesUnordered<BoxFuture<'static, Option<Source>>>;

/// The one source that can supply `image`, its scope held, or `None`.
///
/// The rules:
///
/// 1. **Who is asked.** Every connected provider `reach` reaches whose
///    capabilities keep a Store or list a registry, the only providers
///    that can hold anything; and every remote daemon's record `reach`
///    reaches whose remote is neither in `ignore` nor this daemon
///    itself — the identity it is.
/// 2. **Asked alike, all at once.** Each provider is asked by opening an
///    `images::read` scope with the image, references and all; each
///    remote daemon by opening its own `images::read` scope with the
///    image and `ignore`, this daemon's identity added, on the
///    connection [`daemons::connect`] holds or opens for the record —
///    the routes found as for every relay. A record that will not
///    connect is a no.
/// 3. **The first held answers.** The first scope answered held is the
///    source, and is returned at once. Every other is cancelled: the
///    scopes still opening go on in a task of their own, and each
///    answered held after the first is cancelled as it answers.
/// 4. **Everything else is no.** A not found, an unsupported, a
///    forbidden, an error and a scope that would not open are all no;
///    no scope answering held is `None`, after the last has answered.
pub async fn locate(daemon: &Arc<Daemon>, image: &Image, reach: &Reach, ignore: &[Identity]) -> Option<Source> {
    let mut asking = asks(daemon, image, reach, ignore).await;
    // Rules 3 and 4.
    while let Some(answered) = asking.next().await {
        let Some(source) = answered else {
            continue;
        };
        cancel_rest(asking);
        return Some(source);
    }
    None
}

/// Rules 1 and 2 of [`locate`]: every ask, made and not yet awaited,
/// so that all of them run at once as the set is taken from. Each
/// answers its source when its scope answered held, and `None` for
/// everything else, rule 4.
pub async fn asks(daemon: &Arc<Daemon>, image: &Image, reach: &Reach, ignore: &[Identity]) -> Asks {
    // Rule 1.
    let own = daemon.authenticator.own().cloned();
    let asking: Asks = FuturesUnordered::new();
    let frame = Arc::new(request::Frame(oci::has::request::Request { image: image.clone() }));
    for identity in daemon.live.connected_providers().await {
        let reached = match reach {
            Reach::Full { except } => except.as_ref() != Some(&identity),
            Reach::Granted { providers, .. } => providers.contains(&identity),
        };
        if !reached {
            continue;
        }
        let supplies = daemon
            .live
            .capabilities(&identity)
            .await
            .is_some_and(|info| info.capabilities.images_store || !info.capabilities.images_registries.is_empty());
        if !supplies {
            continue;
        }
        let Some(handle) = daemon.live.provider(&identity).await else {
            continue;
        };
        let frame = Arc::clone(&frame);
        // Rule 2.
        asking.push(
            async move {
                let scope = execute::execute(&handle, &frame).await.ok()?;
                Some(Source { holder: Holder::Provider(identity), scope })
            }
            .boxed(),
        );
    }
    let records = match reach {
        Reach::Full { .. } => recorded(daemon).await,
        Reach::Granted { daemons, .. } => daemons.clone(),
    };
    let mut passed: Vec<Identity> = ignore.to_vec();
    passed.extend(own.clone());
    let passed = Arc::new(passed);
    for record in records {
        if ignore.contains(&record.daemon) || own.as_ref() == Some(&record.daemon) {
            continue;
        }
        let daemon = Arc::clone(daemon);
        let image = image.clone();
        let passed = Arc::clone(&passed);
        // Rule 2.
        asking.push(
            async move {
                let peer = daemons::connect(&daemon, &record).await.ok()?;
                let frame = daemon_request::Frame { image, ignore: passed.to_vec(), relay: None };
                let scope = daemon_execute::execute(&peer.connected.handle, &frame).await.ok()?;
                Some(Source { holder: Holder::Daemon(record.id), scope })
            }
            .boxed(),
        );
    }
    asking
}

/// Rule 3 of [`locate`], for the asks left once an answer is had: they
/// go on in a task of their own, and each that answers held is
/// cancelled as it answers, so no scope is left held with nobody to
/// take from it.
pub fn cancel_rest(mut asking: Asks) {
    tokio::spawn(async move {
        while let Some(answered) = asking.next().await {
            if let Some(late) = answered {
                let _ = late.scope.cancel().await;
            }
        }
    });
}

/// Every remote daemon's record: none where the store cannot be read,
/// so that a failing store leaves the providers to answer.
async fn recorded(daemon: &Arc<Daemon>) -> Vec<Record> {
    let Ok(mut conn) = daemon.store.acquire().await else {
        return Vec::new();
    };
    store::daemons::all(&mut conn).await.unwrap_or_default()
}
