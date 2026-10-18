//! One container connection, handshake to end.
//!
//! [`run`] is the whole of it, on a task of its own per connection:
//! the scope; the container's startup packet read and answered,
//! [`read_startup`]; the database [`dial`]led, [`secure`]d when the
//! mode says, and the daemon's own [`handshake`] toward it; the
//! database's answer forwarded; then the [`relay`], unread, until
//! either end ends. A [`cancel`] request is its own short connection.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod auth;
mod cancel;
mod dial;
mod relay;
mod run;
mod startup;
mod tls;

pub use auth::*;
pub use cancel::*;
pub use dial::*;
pub use relay::*;
pub use run::*;
pub use startup::*;
pub use tls::*;
