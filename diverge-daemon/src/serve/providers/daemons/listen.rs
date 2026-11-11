//! Listening for the messages sent to the daemon, through every
//! connected provider it is messageable on.

use std::sync::Arc;

use tokio::sync::broadcast;

use diverge_sdk::daemon::endpoints::providers::daemons::listen::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::listen::server::response::{Frame, Message};
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Send the messages as they are taken, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let request::Frame { relay: _ } = frame;
    if let Err(error) = serve(&scope, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** One `Forbidden` with no providers `listen` grant
///    over any provider.
/// 2. **Listening.** The scope reads the daemon's
///    [`Listening`](crate::listening::Listening): the ring fed by the
///    listen the daemon holds on every connected provider whose record
///    says `messageable`. The scope opens no provider's listen and ends
///    none; a daemon messageable on no connected provider is one whose
///    listeners hear nothing.
/// 3. **From its entering.** It hears the messages taken after it
///    entered — after the newest number as it began — and none before,
///    though the ring keeps them.
/// 4. **At its own pace.** At every newer number it sends the messages
///    after the last it sent, in order, each as the client takes the
///    one before; one that fell out of the ring meanwhile is first told
///    how many it missed, as `Skipped`. No other listener waits on it,
///    and it waits on none.
/// 5. **The end.** The client's cancel, or its connection ending.
/// 6. **Reached.** It sends only the messages that came through a
///    provider a `listen` grant reaches; the cursor passes the others
///    by. The reach is read from the records when it enters, and
///    again at every word of `Kind::Providers` — and at a lag of the
///    words, which may have hidden one — so a message taken after a
///    record's change is judged as the grants then reach.
async fn serve(scope: &ScopeHandle, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_daemons::holds(&standing, Over::Listen) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // Rules 2 and 3: the newest number read as it is subscribed to, so
    // nothing taken after its number is unheard.
    let listening = &daemon.live.listening;
    // Rule 6: subscribed to the words before the reach is read, so no
    // change after the read goes unheard.
    let mut words = daemon.live.changes();
    let mut reached = judge::providers_daemons::reached(&standing, Over::Listen, &daemon.providers.all());
    let mut latest = listening.latest();
    let mut cursor = *latest.borrow_and_update();
    loop {
        // Rule 4.
        let (skipped, messages) = listening.after(cursor).await;
        if skipped > 0 {
            reply::reply(scope, &Frame::Skipped { count: skipped }).await;
        }
        for (number, kept) in messages {
            // Rule 6.
            if !reached.contains(&kept.provider) {
                cursor = number;
                continue;
            }
            let message = Message { provider: kept.provider.clone(), from: kept.from.clone(), body: kept.body.clone() };
            reply::reply(scope, &Frame::Message(message)).await;
            cursor = number;
        }
        tokio::select! {
            changed = latest.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            // Rule 6.
            word = words.recv() => {
                let again = match word {
                    Ok(kind) => kind == Kind::Providers,
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                };
                if again {
                    reached = judge::providers_daemons::reached(&standing, Over::Listen, &daemon.providers.all());
                }
            }
            // Rule 5.
            () = files::cancelled(scope) => return Ok(()),
        }
    }
}
