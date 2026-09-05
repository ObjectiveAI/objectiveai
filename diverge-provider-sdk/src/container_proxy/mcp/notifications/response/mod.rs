//! What the server sends on `/mcp/notifications`: one notification
//! per frame. The container sends nothing, so there is no `request`.

mod frame;

pub use frame::*;
