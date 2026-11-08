//! The image endpoints, served: the check against the provider's
//! [`ImageChecker`](crate::protocol::image_checker::ImageChecker), the
//! Store's seven against its
//! [`ImageManager`](crate::protocol::image_manager::ImageManager), and
//! the pull against its [`OciStore`](crate::protocol::oci_store::OciStore).

pub mod build;
pub mod check;
pub mod delete;
pub mod edit;
pub mod hash;
pub mod list;
pub mod oci;
pub mod push;
pub mod stat;
