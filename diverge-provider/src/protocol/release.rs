//! A release that happens once, whichever way its owner ends.

use std::future::Future;
use std::pin::Pin;

/// Something held that must be given back — a volume's hold, a
/// container to stop, a directory entry to take out — as the future
/// that gives it back, run exactly once.
///
/// The owner runs it where its teardown wants it, with
/// [`now`](Self::now); an owner that never gets there — a handler
/// that panicked, a task aborted, a future dropped mid-await — drops
/// it instead, and the drop hands the same future to the runtime to
/// finish on a task of its own. A drop cannot await, and a tokio lock
/// is what most of these releases take, which is why the release is
/// spawned and not run in place.
///
/// # The rules
///
/// 1. The future is built when the hold is taken and does nothing
///    until it is first polled: building one releases nothing.
/// 2. [`now`](Self::now) polls it to completion and then forgets it,
///    so a release run is never run again.
/// 3. A [`now`](Self::now) cancelled part way leaves the future where
///    it was, part polled, and the drop that follows spawns it to
///    carry on from there — never to start over, so nothing is given
///    back twice.
/// 4. A drop with the future still held spawns it on the current
///    runtime. With no runtime — the process itself ending — there is
///    nothing left to give anything back to, and the drop does
///    nothing.
pub struct Release {
    future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl Release {
    /// Hold `future` as the release, to run at [`now`](Self::now) or
    /// at the drop, whichever is first.
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Release {
            future: Some(Box::pin(future)),
        }
    }

    /// Release here, and wait for it; see rules 2 and 3.
    pub async fn now(mut self) {
        if let Some(future) = self.future.as_mut() {
            future.as_mut().await;
            self.future = None;
        }
    }
}

impl Drop for Release {
    /// Rule 4: the release not run yet, run on a task of its own.
    fn drop(&mut self) {
        if let Some(future) = self.future.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            runtime.spawn(future);
        }
    }
}

impl std::fmt::Debug for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Release").field("pending", &self.future.is_some()).finish()
    }
}
