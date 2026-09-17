//! The provider, running: everything built once, every connection
//! handed to the SDK, and the stop.
//!
//! The SDK neither listens nor dials the protocol socket: it takes a
//! finished WebSocket, as an accepted upgrade or a dialled stream,
//! and serves it with one call of its `handle` per connection. So
//! this module owns both ends of the socket. [`listen`] accepts a
//! WebSocket upgrade at any path on the configured port, on every
//! interface, and each upgrade is one connection the peer
//! authenticates with its first frame; [`dial`] connects to one peer
//! of `clients.unbrokered`, presents the configured key as the first
//! frame, serves the connection for as long as it lasts, and dials
//! again five seconds after it ends or fails, for the provider's
//! life. [`Provider`] is what every connection is served with — the
//! deployer, the volumes, the image checker, the registry, and the
//! one directory of running containers — built once by
//! [`Provider::start`]; [`run`] builds it, listens and dials, waits
//! for Ctrl-C or SIGTERM, and stops: the listener drained, the dials
//! ended, every container and loop mount of this provider swept away,
//! and the registry and the tunnel gone with the provider. [`Error`]
//! is why the provider could not start or could not listen, the one
//! report a failed start gets; the provider prints nothing else.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod dial;
mod error;
mod listen;
mod provider;
mod run;

pub use dial::*;
pub use error::*;
pub use listen::*;
pub use provider::*;
pub use run::*;
