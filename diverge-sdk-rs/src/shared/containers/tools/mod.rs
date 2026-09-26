//! The tools a container declares, asked of the caller.
//!
//! A program answers its registration with the tools it depends on
//! — each a [`Tool`]: a tool container the caller runs, by its image,
//! its limits and its arguments, and serves to the program under a
//! name. The proxy carries the list on `Begun`; the provider, when
//! the list is not empty, opens one channel on the run scope with
//! [`request::Request`], the list verbatim, and the caller answers
//! with one [`response::Frame`] — deployed, or an error in its own
//! words — then the channel finishes. The run goes on to its id on a
//! deploy and ends, the container stopped, on anything else. A
//! container that declares nothing is never asked about.
//!
//! The provider carries and does not read the list; what the caller
//! does with it — which mounts each tool gets, whether a tool's own
//! tools are deployed in turn, how the container reaches them — is
//! the caller's.

mod tool;

pub mod request;
pub mod response;

pub use tool::*;
