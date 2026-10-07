//! The requests over volumes, served: [`create`], [`get`], [`list`],
//! [`delete`], [`edit`], [`stat`], [`filetree`], [`download`],
//! [`upload`] and [`transfer`]. A volume is its provider's; the
//! daemon keeps no record of one, so every handler finds the volume
//! by asking the provider — [`locate`] — and judges it by what the
//! provider said and which records of the daemon's mount it.

mod locate;

pub use locate::*;

pub mod create;
pub mod delete;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod stat;
pub mod transfer;
pub mod upload;
