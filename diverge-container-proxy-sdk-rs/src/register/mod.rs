//! `POST /register`: the arguments, told to the program once.
//!
//! The proxy sends the [`request::Request`] JSON, the arguments the
//! container was made with, before anything else it asks of the
//! program — an agent's first loop, a tool server's first exchange.
//! A `2xx` is the arguments held for the container's life; a
//! non-`2xx` is the image refusing them, in its own words, and the
//! proxy answers the provider's begin with them. The program refuses
//! a second registration whatever it carries
//! (`{"kind":"registered"}`, `409`), and an agent's `/run` before the
//! first (`{"kind":"unregistered"}`).

pub mod request;
