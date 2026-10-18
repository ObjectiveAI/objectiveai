//! A mounted entry's attributes, changed: the mount, the path, and
//! which of the mode, the owner, the group and the times to set,
//! answered with one message — ok, or why not — then the close. What
//! a `chmod(2)`, a `chown(2)` and a `utimensat(2)` become.

pub mod request;
pub mod response;

mod attrs;

pub use attrs::*;
