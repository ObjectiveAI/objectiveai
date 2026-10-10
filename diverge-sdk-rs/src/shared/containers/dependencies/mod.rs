//! The dependencies a container declares, asked of the caller: each a
//! tool container, as a template the caller deploys unread.
//!
//! An AGENT container's program answers its registration with the
//! tools it depends on — each a [`Template`]: a tool container the
//! caller runs, by its image, its limits and its arguments; the
//! agent's own files and directories the caller serves live into it;
//! which database it gets; the grants its account holds; and the name
//! the caller serves it to the program under. The proxy carries the
//! list on the agents begin's `Begun`; the provider, when the list is
//! not empty, opens one channel on the run scope with
//! [`request::Request`], the list verbatim, and the caller answers
//! with one [`response::Frame`] — deployed, or an error in its own
//! words — then the channel finishes. The run goes on to its id on a
//! deploy and ends, the container stopped, on anything else. An agent
//! that declares nothing is never asked about.
//!
//! A TOOL container has no dependencies: its program may answer its
//! registration with a list, and its proxy ignores it; its begin's
//! `Begun` carries nothing, and its run scope has no such channel.
//! Only an agent has dependencies.
//!
//! The provider carries and does not read the list; everything a
//! caller needs to deploy a dependency is in it, as fields a program
//! acts on and not as words a person reads — which is the difference
//! between a dependency tool template and the tool templates a daemon
//! keeps for itself, and why one travels inside an image.

mod database;
mod mount;
mod template;

pub mod request;
pub mod response;

pub use database::*;
pub use mount::*;
pub use template::*;
