//! Changing a role: a patch to its description and its grants. One
//! request, one answer. A client names a role and a patch — see
//! [`patch`](crate::daemon::patch) — to its editable members; the
//! daemon applies it, whole or not at all, and answers that the role
//! is as the result states, that no role has the name, forbidden, or
//! that it failed, and the scope finishes. Every account holding the
//! role is judged by the new grants from then on. A role's name does
//! not change: a role is reached by its name alone, so a new name is a
//! new role.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
