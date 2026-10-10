//! The requests over volumes, served: [`create`], [`get`], [`list`],
//! [`delete`], [`edit`], [`stat`], [`filetree`], [`download`],
//! [`upload`], [`transfer`], [`tag`] and [`untag`]. A volume is its
//! provider's; the daemon keeps no record of one but its tags, so
//! every handler finds the volume by asking the provider —
//! [`locate`] — and judges it by what the provider said, which
//! records of the daemon's mount it, and the tags on it.

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
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;
