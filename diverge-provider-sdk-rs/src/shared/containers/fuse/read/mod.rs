//! A piece of a mounted file, read: the mount, the path, an offset
//! and a length, answered with one message — the bytes there, or
//! that there is no file — then the close.

pub mod request;
pub mod response;
