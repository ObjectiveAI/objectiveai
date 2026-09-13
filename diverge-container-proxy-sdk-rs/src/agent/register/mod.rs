//! `POST /register`: the agent, told to the program once.
//!
//! The proxy sends the [`request::Request`] JSON, the agent value the
//! container was made with, before any loop. A `2xx` is the agent
//! held for the container's life; a non-`2xx` is the image refusing
//! it, in its own words, and the proxy answers the provider's begin
//! with them. The program refuses a second registration whatever it
//! carries (`{"kind":"registered"}`, `409`), and a `/run` before the
//! first (`{"kind":"unregistered"}`).

pub mod request;
