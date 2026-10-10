//! The eleven volume endpoints, served, each against the provider's
//! [`VolumeManager`](crate::protocol::volume_manager::VolumeManager).

pub mod create;
pub mod create_capacity;
pub mod delete;
pub mod edit;
pub mod edit_capacity;
pub mod filetree;
pub mod list;
pub mod read;
pub mod serve;
pub mod stat;
pub mod write;
