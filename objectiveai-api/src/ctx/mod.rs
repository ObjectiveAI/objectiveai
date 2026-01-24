//! Request context and extensions for dependency injection.
//!
//! The context system allows per-request state and customization through
//! the `ContextExt` trait. This enables features like BYOK (Bring Your Own Key)
//! support where users can provide their own upstream API keys.

mod ctx;
mod ctx_ext;
mod default_ctx_ext;

pub use ctx::*;
pub use ctx_ext::*;
pub use default_ctx_ext::*;
