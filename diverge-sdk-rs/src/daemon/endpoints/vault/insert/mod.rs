//! Inserting one value into a vault key. One request, one answer. A
//! client names a key and gives a value — and, when it means to
//! replace one the key holds, which slot; the daemon answers that the
//! key holds it — the key made with it as its first value where the
//! vault had no such key, the value appended as one more slot where
//! it had and no slot was named, the slot's value replaced where one
//! was — that no such key or slot is there, that the slot is locked
//! to a running container and was left as it is, forbidden, or that
//! it failed, and the scope finishes. A key some agent or tool binds
//! is inserted into like any other: the next activation of that agent
//! or tool may be handed the new value.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
