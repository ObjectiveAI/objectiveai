//! Responses to [`super::client_request::Request`]s — `ok` for empty
//! success, `error` carrying a code + message for failure. The `id`
//! echoes the request's `id` so the client can correlate replies to
//! in-flight requests.

mod response;
pub use response::*;
