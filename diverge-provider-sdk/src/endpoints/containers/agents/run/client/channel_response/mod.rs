//! What a caller sends back during a run, one module per kind of
//! channel the provider opens.
//!
//! [`oci`] serves the image when it is the caller's to serve;
//! [`authorize`] answers whether a connector may join; [`write_bytes`]
//! streams the content of a file being written; [`fetch_file`] and
//! [`fetch_directory`] hand over mounted content the provider does not
//! hold. The rest answer the CONTAINER, relayed: [`postgres`] is what
//! its database said, [`command`] the items its command produced, the
//! five `vault_*` what its vault answered, and the five `mcp_*` what
//! the caller's MCP servers answered its tool calls with.

pub mod authorize;
pub mod command;
pub mod fetch_directory;
pub mod fetch_file;
pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod oci;
pub mod postgres;
pub mod vault_delete;
pub mod vault_get;
pub mod vault_lock;
pub mod vault_set;
pub mod vault_unlock;
pub mod write_bytes;
