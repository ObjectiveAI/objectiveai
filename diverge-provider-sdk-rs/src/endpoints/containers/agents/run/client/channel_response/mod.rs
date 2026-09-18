//! What a caller sends back during a run, one module per kind of
//! channel the provider opens.
//!
//! [`oci_has`] says whether the caller holds the image, and
//! [`oci_manifest`] and [`oci_blob`] hand it over when the provider
//! takes it from there; [`authorize`] answers whether a connector
//! may join; [`write_bytes`]
//! streams the content of a file being written. The rest answer the
//! CONTAINER, relayed: [`postgres`] is what
//! its database said, [`command`] the items its command produced, the
//! five `vault_*` what its vault answered, the five `mcp_*` what the
//! caller's MCP servers answered its tool calls with, and the seven
//! `fuse_*` what the files and directories it mounted live hold, what
//! an entry of them is, and whether a change to one took.

pub mod authorize;
pub mod command;
pub mod fuse_list;
pub mod fuse_mkdir;
pub mod fuse_read;
pub mod fuse_remove;
pub mod fuse_rename;
pub mod fuse_stat;
pub mod fuse_write;
pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod oci_blob;
pub mod oci_has;
pub mod oci_manifest;
pub mod postgres;
pub mod vault_delete;
pub mod vault_get;
pub mod vault_lock;
pub mod vault_set;
pub mod vault_unlock;
pub mod write_bytes;
