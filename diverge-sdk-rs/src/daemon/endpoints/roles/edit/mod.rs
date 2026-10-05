//! Changing a role. One request, one answer. A client names a role and
//! what is to change — its description, its grants, each as it is to
//! be, taken away, or left as it is; the daemon answers that the role
//! is as the request states, that no role has the name, forbidden, or
//! that it failed, and the scope finishes. The request is applied whole
//! or not at all, and every account holding the role is judged by the
//! new grants from then on. A role's name does not change: a role is
//! reached by its name alone, so a new name is a new role.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
