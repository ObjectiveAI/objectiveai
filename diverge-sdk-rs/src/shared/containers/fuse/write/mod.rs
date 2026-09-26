//! A piece of a mounted file, written: the mount, the path, an
//! offset and the bytes, in place, answered with one message — ok,
//! or why not — then the close.

pub mod request;
pub mod response;
