//! One HTTP request to an agent's frontend.
//!
//! A client names an agent and the request's head; the daemon sends it
//! to the frontend the agent's program serves beneath `/frontend` (see
//! [`frontend`](crate::shared::containers::frontend)), takes the body on
//! the client's channels, and sends the answer — its head, then its
//! body — and finishes; or answers that nothing is the one named,
//! forbidden, that the agent's image declares no frontend, or that it
//! failed, and finishes. An agent's container that is not running is started for the request, and held up while it is answered.
//! Split by who SENDS: the request and the channels are in [`client`],
//! the answer in [`server`].

pub mod client;
pub mod server;
