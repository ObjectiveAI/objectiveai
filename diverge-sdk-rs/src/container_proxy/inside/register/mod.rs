//! `POST /register`: the arguments and the template, told to the
//! program once; the dependencies and the secrets' declaration,
//! answered once.
//!
//! The proxy sends the [`request::Request`] JSON — the arguments the
//! container was made with and the id of the template it was made
//! from — before anything else it asks of the program — an agent's
//! first loop, a tool server's first exchange. A `2xx` is the
//! arguments held for the container's life, and its body is the
//! [`response::Response`] JSON: the dependency tool templates the
//! program depends on, and the secrets it takes, each by name to its
//! mode. The proxy carries both to the provider, the provider has the
//! caller deploy the dependencies and resolve the secrets, and the
//! first values come back down `POST /secrets` before the container's
//! id goes out. A non-`2xx` is the image refusing the arguments, in
//! its own words, and the proxy answers the provider's begin with
//! them; so is a `2xx` whose body is not the response. The program
//! refuses a second registration whatever it carries
//! (`{"kind":"registered"}`, `409`), and an agent's `/run` before the
//! first (`{"kind":"unregistered"}`).

pub mod request;
pub mod response;
