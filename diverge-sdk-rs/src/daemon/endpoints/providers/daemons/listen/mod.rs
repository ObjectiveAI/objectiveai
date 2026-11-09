//! Listening for the direct messages sent to the daemon. A client asks,
//! and the daemon sends every message sent to its identity through any
//! provider it is connected to, one response each, until the client
//! cancels, the one channel it opens on the scope.
//!
//! The rules:
//!
//! 1. **Every connected provider.** The messages are those of every
//!    connected provider's own `daemons::listen`, merged: at most one
//!    listen per provider, however many clients listen.
//! 2. **Every listener hears every message.** Each client listening
//!    reads the same merged messages, at its own pace; none waits for
//!    another.
//! 3. **From now on.** A listener hears the messages the daemon takes
//!    after it opened, and none before: there is no history.
//! 4. **Kept a while.** The daemon keeps the last messages it took for
//!    the listeners still reading them; a listener that falls further
//!    behind than the daemon keeps is told how many it missed, as
//!    [`Skipped`](server::response::Frame::Skipped), and reads on.
//! 5. **Judged over providers.** A listen needs the `listen` action of
//!    the providers grant.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`].

pub mod client;
pub mod server;
