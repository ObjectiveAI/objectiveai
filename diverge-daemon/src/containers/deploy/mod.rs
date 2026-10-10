//! An agent's declared dependencies, deployed then and there.
//!
//! An agent's program answers its registration with the tools it
//! depends on, each a dependency tool template, and the provider asks
//! the daemon to deploy them before the container's id is out, the id
//! told with the ask. [`deploy`] starts every one at once, each a tool
//! container of the agent's own: on any connected provider, tried in
//! random order; its mounts the agent's paths served live from the
//! agent's container; its database scope the one its template names;
//! its requests to the daemon judged by its template's grants; served
//! to the agent under the declared name. The first that cannot be
//! deployed is the run's error, and the rest are stopped. A
//! dependency lives for as long as the agent's container does, and
//! no longer. A tool container declares no dependencies and is never
//! deployed for.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod deploy;

pub use deploy::*;
