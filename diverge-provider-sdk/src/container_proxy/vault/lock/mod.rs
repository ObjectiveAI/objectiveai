//! `/vault/lock/{channel}`: kind `8` on `/requests`, a key and a TTL, its lock held,
//! answered with one message — ok, or error — then the close.

pub mod request;
pub mod response;
