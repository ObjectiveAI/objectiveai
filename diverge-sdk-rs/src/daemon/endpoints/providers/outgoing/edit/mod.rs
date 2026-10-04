//! Changing how an outgoing provider is dialled. One request, one
//! answer. A client names a provider of its own by address and gives a
//! mode anew; the daemon answers that the provider has it, that no
//! provider of the caller's has that address, or that it failed, and
//! the scope finishes. The mode is replaced whole, which is how a
//! credential rotates; the address, being the identity, does not change
//! — a provider at another address is another add.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
