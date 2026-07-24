//! A live per-channel stream — `GET /channels/{id}`.
//!
//! Two modes, decided by [`ChannelStream::open`]'s `secret` argument:
//! - **Accept** (`None`): opening the stream of a PENDING offer IS the
//!   accept. The daemon's first frame delivers the owner secret
//!   (`S_owner`); the stream is the channel's LIVENESS ANCHOR —
//!   dropping the handle disconnects, and the daemon closes the
//!   channel (terminal).
//! - **Observer** (`Some(S_pub | S_owner)`): a pure watcher — silent
//!   until the channel closes, then one `closed` frame and the stream
//!   ends. Observer drops close nothing.
//!
//! There is deliberately NO reconnect: an accept stream's disconnect
//! already closed the channel, and an observer that reconnects gets an
//! immediate `closed` frame from the terminal state anyway.

use futures::StreamExt;

use super::{ChannelStreamEvent, Error};

/// A live per-channel stream handle. See the module docs for the two
/// modes. Dropping the handle disconnects (which, for an accept-mode
/// stream, CLOSES the channel).
pub struct ChannelStream {
    /// `Some(S_owner)` for an accept-mode open; `None` for observers.
    secret: Option<String>,
    /// Flips to `true` when the `closed` frame arrives OR the
    /// transport ends.
    closed: tokio::sync::watch::Receiver<bool>,
    /// The frame pump; aborted on drop, which drops the transport.
    pump: tokio::task::JoinHandle<()>,
}

impl ChannelStream {
    /// Open `GET {base}/channels/{id}`: accept mode when `secret` is
    /// `None`, observer mode otherwise (the channel secret rides the
    /// `X-OBJECTIVEAI-CHANNEL-SECRET` header). `signature` is the
    /// daemon auth header, as everywhere.
    pub(crate) async fn open(
        base_url: &str,
        signature: Option<&str>,
        channel_id: &str,
        secret: Option<&str>,
    ) -> Result<Self, Error> {
        let url = format!(
            "{}/channels/{}",
            base_url.trim_end_matches('/'),
            channel_id
        );
        let client = reqwest::Client::builder().build()?;
        let mut request = client.get(url).header("Accept", "text/event-stream");
        if let Some(signature) = signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        let accepting = secret.is_none();
        if let Some(secret) = secret {
            request = request.header("X-OBJECTIVEAI-CHANNEL-SECRET", secret);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status {
                reqwest::StatusCode::NOT_FOUND => Error::NotFound,
                reqwest::StatusCode::CONFLICT => Error::AlreadyAccepted,
                reqwest::StatusCode::UNAUTHORIZED => Error::Unauthorized,
                _ => Error::Status(status),
            });
        }
        use eventsource_stream::Eventsource;
        let mut events = response.bytes_stream().eventsource();
        // Accept mode: the FIRST frame must deliver the owner secret —
        // consumed here so the caller holds the capability the moment
        // the handle exists.
        let owner_secret = if accepting {
            loop {
                let Some(event) = events.next().await else {
                    return Err(Error::StreamEnded);
                };
                let Ok(event) = event else {
                    return Err(Error::StreamEnded);
                };
                match serde_json::from_str::<ChannelStreamEvent>(&event.data) {
                    Ok(ChannelStreamEvent::Secret { secret }) => break Some(secret),
                    Ok(_) => return Err(Error::UnexpectedFrame),
                    // Skip unparseable frames (comments/keepalives).
                    Err(_) => continue,
                }
            }
        } else {
            None
        };
        let (closed_tx, closed_rx) = tokio::sync::watch::channel(false);
        let pump = tokio::spawn(async move {
            while let Some(Ok(event)) = events.next().await {
                if let Ok(ChannelStreamEvent::Closed) =
                    serde_json::from_str::<ChannelStreamEvent>(&event.data)
                {
                    break;
                }
            }
            // `closed` frame OR transport end — either way the stream
            // is over.
            let _ = closed_tx.send(true);
        });
        Ok(Self {
            secret: owner_secret,
            closed: closed_rx,
            pump,
        })
    }

    /// The owner secret (`S_owner`) — `Some` iff this is an
    /// accept-mode stream. The per-channel capability for
    /// `channels logs reply|list|open|subscribe`.
    pub fn secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    /// Whether the channel has closed (or the transport ended).
    pub fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }

    /// Resolve when the channel closes (the `closed` frame) or the
    /// transport ends, whichever comes first.
    pub async fn closed(&self) {
        let mut rx = self.closed.clone();
        // wait_for resolves immediately if already true.
        let _ = rx.wait_for(|closed| *closed).await;
    }
}

impl Drop for ChannelStream {
    fn drop(&mut self) {
        // Aborting the pump drops the transport: for an accept-mode
        // stream the daemon closes the channel.
        self.pump.abort();
    }
}
