//! What a caller sends back on the channels a provider opens.
//!
//! [`oci`] serves the image when it is the caller's to serve;
//! [`postgres`] is the far end of the plugin's database connection.
//!
//! One module per kind of channel, each naming its own type `Frame`.
//! Nothing is re-exported upward: the module is the only thing telling
//! two types called `Frame` apart, so it has to stay in the path.
//!
//! [`oci`]'s frame is an ALIAS of
//! [`http::response::Frame`](crate::shared::http::response::Frame) —
//! the head-then-body split is a fact about HTTP rather than about
//! that channel. [`postgres`]'s is its own type, because a byte stream
//! is not an HTTP anything.

pub mod oci;
pub mod postgres;
