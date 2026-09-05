//! `/mcp/list-resources/{channel}`: kind `1` on `/requests`, answered with exactly one message,
//! then the close. Both sides are the shared exchange's own shapes,
//! re-exported: [`request::Request`] is the ask, [`response::Frame`]
//! a message on the path.

pub mod request;
pub mod response;
