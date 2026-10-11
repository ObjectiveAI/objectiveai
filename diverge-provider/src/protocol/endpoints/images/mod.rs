//! The five image endpoints, served: the check against the provider's
//! [`ImageChecker`](crate::protocol::image_checker::ImageChecker), and
//! the store's four against its
//! [`ImageManager`](crate::protocol::image_manager::ImageManager).

pub mod build;
pub mod check;
pub mod delete;
pub mod list;
pub mod stat;
