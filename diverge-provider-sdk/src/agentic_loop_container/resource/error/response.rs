//! The container taking the failure.

/// The 2xx body of one `POST /resource/{identity}/error`.
///
/// An alias of the delivery's own response, because the taking is
/// the same shape whatever was taken — a chunk, the completion, or
/// the failure: `{"type":"received"}`, and nothing else a success
/// could say.
///
/// An alias rather than a re-export because this module is real:
/// the path says the error route's response lives here, and it
/// does; what it points at is right there in the signature.
pub type Response = super::super::Response;
