//! `/mcp/notifications/{channel}`: kind `4` on `/requests`, answered with one message per notification, an error last,
//! then the close. Both sides are the shared exchange's own shapes,
//! re-exported: [`request::Request`] is the ask, [`response::Frame`]
//! a message on the path.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
