//! Sending one direct message, through the daemon's providers. A client
//! names the identity and the body, and the daemon sends it through one
//! provider where that identity listens, and answers its fate.
//!
//! The rules:
//!
//! 1. **The daemon chooses the provider.** The client names none. The
//!    daemon sends through a provider whose daemons list holds the
//!    identity listening, that the account's providers grant reaches
//!    with `message`, choosing the one with the most of its rate left.
//! 2. **Never refused for another send.** A provider holds one send
//!    per connection; the daemon queues its own sends, and a message is
//!    never refused because another is being sent.
//! 3. **Waits for room.** When every provider it could go through is
//!    over its rate, the message waits until one has room again.
//! 4. **The answer.** Exactly one, when the fate is known: delivered —
//!    the receiver's reader took it — not listening, not found,
//!    forbidden, or the error. A cancel in time is the finish alone.
//!
//! Split by who SENDS, as everywhere else.

pub mod client;
pub mod server;
