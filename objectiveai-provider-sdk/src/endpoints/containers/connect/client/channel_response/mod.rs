//! What a connector sends back during a connection.
//!
//! [`write_bytes`] is the only one. A provider opens a channel on a
//! connector for exactly one reason — to collect the content of a file
//! the connector asked to write.

pub mod write_bytes;
