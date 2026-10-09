//! The tools a container declares, asked of the caller.
//!
//! An AGENT container's program answers its registration with the
//! tools it depends on — each a [`Tool`]: a tool container the caller
//! runs, by its image, its limits and its arguments, and serves to
//! the program under a name. The proxy carries the list on the agents
//! begin's `Begun`; the provider, when the list is not empty, opens
//! one channel on the run scope with [`request::Request`], the list
//! verbatim, and the caller answers with one [`response::Frame`] —
//! deployed, or an error in its own words — then the channel
//! finishes. The run goes on to its id on a deploy and ends, the
//! container stopped, on anything else. An agent that declares
//! nothing is never asked about.
//!
//! A TOOL container has no dependencies: its program may answer its
//! registration with a list, and its proxy ignores it; its begin's
//! `Begun` carries nothing, and its run scope has no such channel.
//! Only an agent has tools.
//!
//! The provider carries and does not read the list; what the caller
//! does with it — which mounts each tool gets, how the agent reaches
//! them — is the caller's.

mod tool;

pub mod request;
pub mod response;

pub use tool::*;
