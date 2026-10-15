//! Changing a volume: how many bytes it reserves, its mode, or both.
//! One request, one answer. A client names a volume and what is to
//! change; the daemon asks the provider, and answers that the volume is
//! as the request states, that no volume is the one named, that the
//! provider cannot reserve that many bytes, that the volume holds more
//! than that, that it is held, forbidden, or that it failed, and the
//! scope finishes. Both at once is one edit: a size the provider
//! refuses changes the mode no more than the size. A volume's name does
//! not change: it is the handle, and a mount naming it would name
//! nothing.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
