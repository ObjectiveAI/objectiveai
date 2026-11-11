//! Grants over providers that name none of them.
//!
//! The daemon's providers are its records, each one provider it works
//! with. What a grant may ask over them, in the shapes
//! [`permission`](crate::shared::permission) states: [`Make`], the
//! action that records one; [`Over`], the actions over the records —
//! reading, listing, asking them for images, changing, deleting,
//! enabling and disabling one — reaching by tags;
//! and [`Permission`], one grant's worth of either, or of tagging.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
