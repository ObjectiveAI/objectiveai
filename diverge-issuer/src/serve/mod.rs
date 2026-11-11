//! The issuer, running.
//!
//! [`run`] is the whole of its life: the root locked, the database
//! opened, the port listened on, and the stop. [`Issuer`] is what
//! every request is served with. [`Error`] is why a start failed.
//!
//! The port serves plain HTTP: the OpenID Connect and OAuth
//! endpoints; `POST /stop`, from loopback only, as the daemon and the
//! provider take it — see [`stop`]; and `POST /rotate`, from loopback
//! only, the issuer rotating its signing key itself and signing with
//! the new one at once — see [`rotate`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod issuer;
mod listen;
mod run;
mod rotate_route;
mod shutdown;
mod stop_route;

pub use error::*;
pub use issuer::*;
pub use listen::*;
pub use rotate_route::*;
pub use run::*;
pub use shutdown::*;
pub use stop_route::*;
