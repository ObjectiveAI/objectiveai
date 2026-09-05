//! `/vault/unlock/{channel}`: kind `9` on `/requests`, a key, its lock released,
//! answered with one message — ok, or error — then the close.

pub mod request;
pub mod response;
