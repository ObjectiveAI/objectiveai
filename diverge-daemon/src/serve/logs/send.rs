//! Reading a book, and watching it: the loop the endpoint runs once it
//! knows whose log it is.

use chrono::{DateTime, Utc};
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::watch;

use crate::daemon::Daemon;
use crate::logs::{self, Book, Entry};
use crate::serve::reply;

/// What a logs request says of the span: everything but the type,
/// which is the endpoint's `keep`.
pub struct Span {
    /// The first `logs_index` to read.
    pub from: Option<u64>,
    /// The last `logs_index` to read.
    pub to: Option<u64>,
    /// The latest `created` to read.
    pub created_to: Option<DateTime<Utc>>,
    /// How many items to send at most.
    pub count: Option<u64>,
    /// Whether to watch.
    pub watch: Option<bool>,
}

/// Rules 1 to 3: send every item of `book`'s log the span holds and
/// `keep` lets through, each as `frame` makes it, watching `latest`
/// if asked; `is_cancel` reads a channel request's payload. The
/// handler finishes the scope after.
pub async fn send<B: Book, F: Encode>(
    scope: &ScopeHandle,
    daemon: &Daemon,
    book: B,
    mut latest: watch::Receiver<u64>,
    span: &Span,
    keep: impl Fn(&B::Entry) -> bool,
    frame: impl Fn(B::Entry) -> F,
    is_cancel: fn(&[u8]) -> bool,
) -> Result<(), logs::Error> {
    let watching = span.watch.unwrap_or(false);
    let cap = span.count.unwrap_or(u64::MAX);
    let mut sent = 0u64;
    let mut next = span.from.unwrap_or(1).max(1);
    loop {
        if read_through(scope, daemon, book, span, &keep, &frame, &mut next, &mut sent, cap).await? {
            return Ok(());
        }
        if !watching {
            return Ok(());
        }
        let deadline = span.created_to.and_then(|to| (to - Utc::now()).to_std().ok());
        tokio::select! {
            changed = latest.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            cancelled = cancel(scope, is_cancel) => {
                if cancelled {
                    return Ok(());
                }
            }
            () = wait(deadline) => {
                return Ok(());
            }
        }
    }
}

/// Send every item from `next` through the log's end that the filter
/// yields, moving `next` past what was looked at; `true` when nothing
/// more can come — the count met, `logs_index_to` reached, or an item
/// later than `created_to` seen.
async fn read_through<B: Book, F: Encode>(
    scope: &ScopeHandle,
    daemon: &Daemon,
    book: B,
    span: &Span,
    keep: &impl Fn(&B::Entry) -> bool,
    frame: &impl Fn(B::Entry) -> F,
    next: &mut u64,
    sent: &mut u64,
    cap: u64,
) -> Result<bool, logs::Error> {
    let count = logs::count(&daemon.logs, book).await?;
    let to = span.to.unwrap_or(u64::MAX).min(count);
    if *next <= to {
        for item in logs::read(&daemon.logs, book, *next, to).await? {
            if span.created_to.is_some_and(|bound| item.created() > bound) {
                return Ok(true);
            }
            if keep(&item) {
                reply::reply(scope, &frame(item)).await;
                *sent += 1;
                if *sent >= cap {
                    return Ok(true);
                }
            }
        }
        *next = to + 1;
    }
    Ok(span.to.is_some_and(|bound| count >= bound))
}

/// Whether the client cancelled: the one channel it may open, read
/// and never answered. The inbox closing is not a cancel — the finish
/// is the handler's.
async fn cancel(scope: &ScopeHandle, is_cancel: fn(&[u8]) -> bool) -> bool {
    loop {
        let Some(bytes) = scope.recv_channel_request().await else {
            return std::future::pending().await;
        };
        if let Ok(ClientFrame::ChannelRequest { payload, .. }) = ClientFrame::decode(&bytes)
            && is_cancel(payload)
        {
            return true;
        }
    }
}

/// The clock passing `created_to`, when there is one; never,
/// otherwise.
async fn wait(deadline: Option<std::time::Duration>) {
    match deadline {
        Some(deadline) => tokio::time::sleep(deadline).await,
        None => std::future::pending().await,
    }
}
