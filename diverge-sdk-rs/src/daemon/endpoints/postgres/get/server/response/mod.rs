//! The get response: the mode, forbidden, or a failure. The mode comes
//! back as [`Mode`](crate::daemon::endpoints::postgres::Mode), the very
//! shape the daemon's configuration gives it in, defined beside the
//! family and not repeated here — but for a remote mode's password,
//! which is never answered.

mod frame;

pub use frame::*;
