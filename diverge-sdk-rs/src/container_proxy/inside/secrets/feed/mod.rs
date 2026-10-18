//! `POST /secrets`: the secrets' values, handed to the program.
//!
//! The proxy sends the [`request::Request`] JSON — every secret the
//! program declared, by name to its value — when the caller feeds
//! them: once before the container's id goes out, and at every
//! activation after. A `2xx` is the values applied, and the body is
//! unread; the program answers only once it holds them, since
//! nothing is delivered to it — no message, no MCP exchange — until
//! the call answers. A non-`2xx` is the program refusing the values,
//! in its own words, and is the activation's error: the proxy answers
//! the feed with them, and the caller gives the locked values back.

pub mod request;
