//! `POST /register`: the arguments, told to the program once; the
//! tools, answered once.
//!
//! The proxy sends the [`request::Request`] JSON, the arguments the
//! container was made with, before anything else it asks of the
//! program — an agent's first loop, a tool server's first exchange.
//! A `2xx` is the arguments held for the container's life, and its
//! body is the [`response::Response`] JSON: the tools the program
//! depends on, which the proxy carries to the provider and the
//! provider has the caller deploy before the container's id goes
//! out. A non-`2xx` is the image refusing the arguments, in its own
//! words, and the proxy answers the provider's begin with them; so is
//! a `2xx` whose body is not the response. The program refuses a
//! second registration whatever it carries
//! (`{"kind":"registered"}`, `409`), and an agent's `/run` before the
//! first (`{"kind":"unregistered"}`).

pub mod request;
pub mod response;
