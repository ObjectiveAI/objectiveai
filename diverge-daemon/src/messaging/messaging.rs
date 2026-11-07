//! Each provider's send lane and rate window, as the daemon keeps them.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use diverge_sdk::provider::endpoints::info::server::response::MessagesCapabilities;
use diverge_sdk::shared::Identity;
use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

/// Every provider's lane and window, for the daemon's sends. One per
/// daemon, in its live state. The rules:
///
/// 1. **One lane per provider.** A provider refuses a second send on one
///    connection while one is open, so the daemon's sends through a
///    provider take turns on its lane: a send holds it from before its
///    request until its answer, or its finish, is read. A send never
///    waits on a lane: it takes one that is free ([`claim`](Self::claim))
///    or waits for any to be freed ([`freed`](Self::freed)).
/// 2. **One window per provider.** The daemon counts its own sends
///    through a provider as the provider counts them: a window begins
///    at the first send it counts, lasts the provider's `seconds`, and
///    holds at most its `messages`; a send after it begins another.
///    [`room`](Self::room) is how many more the window holds now.
/// 3. **Counted as it goes.** A send is counted as it goes out
///    ([`count`](Self::count)), and taken back ([`uncount`](Self::uncount))
///    when the provider answers that it did not count it.
/// 4. **Told full.** A provider that answers a send over its rate, the
///    daemon's count notwithstanding, has its window held full until
///    the seconds it named have passed ([`full`](Self::full)).
/// 5. **Freed is told.** Every lane let go, and every window a waiting
///    send waits out, wakes every send waiting for room.
/// 6. **Gone with the provider.** A provider that disconnects has its
///    lane and window dropped ([`disconnected`](Self::disconnected)); a
///    send holding the lane keeps it to its end.
#[derive(Debug, Default)]
pub struct Messaging {
    providers: Mutex<HashMap<Identity, Arc<Provider>>>,
    freed: Arc<Notify>,
}

/// One provider's lane and window.
#[derive(Debug, Default)]
struct Provider {
    lane: Arc<Mutex<()>>,
    window: Mutex<Window>,
}

/// One window, as the daemon counts it (rule 2).
#[derive(Debug, Default, Clone, Copy)]
struct Window {
    /// When it began, if it has.
    began: Option<Instant>,
    /// How many sends it counted.
    count: u32,
    /// Held full until then (rule 4).
    full_until: Option<Instant>,
}

/// A provider's lane, held by one send (rule 1): dropped, it is free,
/// and every send waiting for room is told.
#[derive(Debug)]
pub struct Lane {
    _lane: OwnedMutexGuard<()>,
    freed: Arc<Notify>,
}

impl Drop for Lane {
    fn drop(&mut self) {
        self.freed.notify_waiters();
    }
}

impl Window {
    /// How many more sends it holds at `now`, at `rate`, and when that
    /// next grows, if it is full.
    fn room(&self, rate: MessagesCapabilities, now: Instant) -> (u32, Option<Instant>) {
        if let Some(until) = self.full_until
            && now < until
        {
            return (0, Some(until));
        }
        let length = Duration::from_secs(u64::from(rate.seconds.get()));
        match self.began {
            Some(began) if now < began + length => {
                let left = rate.messages.get().saturating_sub(self.count);
                (left, (left == 0).then_some(began + length))
            }
            _ => (rate.messages.get(), None),
        }
    }
}

impl Messaging {
    /// `provider`'s entry, made at its first use.
    async fn provider(&self, provider: &Identity) -> Arc<Provider> {
        Arc::clone(self.providers.lock().await.entry(provider.clone()).or_default())
    }

    /// How many more sends `provider`'s window holds now, at its `rate`,
    /// and when it next grows when it holds none (rule 2).
    pub async fn room(&self, provider: &Identity, rate: MessagesCapabilities) -> (u32, Option<Instant>) {
        self.provider(provider).await.window.lock().await.room(rate, Instant::now())
    }

    /// `provider`'s lane, when it is free (rule 1).
    pub async fn claim(&self, provider: &Identity) -> Option<Lane> {
        let lane = Arc::clone(&self.provider(provider).await.lane);
        lane.try_lock_owned().ok().map(|guard| Lane { _lane: guard, freed: Arc::clone(&self.freed) })
    }

    /// One send through `provider` counted (rule 3).
    pub async fn count(&self, provider: &Identity, rate: MessagesCapabilities) {
        let entry = self.provider(provider).await;
        let mut window = entry.window.lock().await;
        let now = Instant::now();
        let length = Duration::from_secs(u64::from(rate.seconds.get()));
        match window.began {
            Some(began) if now < began + length => window.count += 1,
            _ => {
                window.began = Some(now);
                window.count = 1;
            }
        }
    }

    /// One send through `provider` taken back: the provider did not
    /// count it (rule 3).
    pub async fn uncount(&self, provider: &Identity) {
        let entry = self.provider(provider).await;
        let mut window = entry.window.lock().await;
        window.count = window.count.saturating_sub(1);
    }

    /// `provider` answered a send over its rate: its window held full
    /// for `seconds` (rule 4).
    pub async fn full(&self, provider: &Identity, seconds: u32) {
        let entry = self.provider(provider).await;
        entry.window.lock().await.full_until = Some(Instant::now() + Duration::from_secs(u64::from(seconds)));
    }

    /// Resolves at the next lane let go (rule 5). Made before the lanes
    /// are looked at, and enabled, so a lane freed between the look and
    /// the wait is not missed.
    pub fn freed(&self) -> Arc<Notify> {
        Arc::clone(&self.freed)
    }

    /// `provider` disconnected (rule 6).
    pub async fn disconnected(&self, provider: &Identity) {
        self.providers.lock().await.remove(provider);
    }
}
