//! The images build response: [`Frame`], a line of the build's output
//! or the one answer that ends it; [`Built`], the image built; and
//! [`Exists`], the image owned already.

mod built;
mod exists;
mod frame;

pub use built::*;
pub use exists::*;
pub use frame::*;
