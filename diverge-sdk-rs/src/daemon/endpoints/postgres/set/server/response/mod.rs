//! The set response: the mode is set, in use, forbidden, or a failure.
//! The containers an `InUse` names come back as
//! [`Connection`](crate::daemon::endpoints::postgres::Connection)s, the
//! very shape a connections list sends them in, defined beside the
//! family and not repeated here.

mod frame;

pub use frame::*;
