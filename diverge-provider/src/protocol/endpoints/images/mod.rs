//! The image endpoints, served: the Store's six against the
//! provider's [`ImageManager`](crate::protocol::image_manager::ImageManager),
//! and the pull — held, served and proven — against its
//! [`OciStore`](crate::protocol::oci_store::OciStore).

pub mod build;
pub mod delete;
pub mod edit;
pub mod list;
pub mod oci;
pub mod push;
pub mod stat;
