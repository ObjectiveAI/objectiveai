//! A mounted file, cut or extended to a length: the mount, the path
//! and the size, answered with one message — ok, or why not — then
//! the close. What a `truncate(2)`, an `O_TRUNC` open and a
//! `ftruncate(2)` become.

pub mod request;
pub mod response;
