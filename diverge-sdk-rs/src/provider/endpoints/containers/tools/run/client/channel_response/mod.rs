//! What a caller sends back during a run, one module per kind of
//! channel the provider opens.
//!
//! [`oci_has`] says whether the caller holds the image and under
//! what name, and [`oci_manifest`] and [`oci_blob`] hand it over when the provider
//! takes it from there; [`write_bytes`] streams the content of a file
//! being written. The rest answer the CONTAINER, relayed: [`postgres`] is
//! what its database said, [`daemon`] the server frames that answer a
//! frame of its daemon connection, and the nine `fuse_*` what the files and
//! directories it mounted live hold, what an entry of them is, and
//! whether a change to one took.

pub mod daemon;
pub mod fuse_list;
pub mod fuse_mkdir;
pub mod fuse_read;
pub mod fuse_remove;
pub mod fuse_rename;
pub mod fuse_setattr;
pub mod fuse_stat;
pub mod fuse_truncate;
pub mod fuse_write;
pub mod oci_blob;
pub mod oci_has;
pub mod oci_manifest;
pub mod postgres;
pub mod write_bytes;
