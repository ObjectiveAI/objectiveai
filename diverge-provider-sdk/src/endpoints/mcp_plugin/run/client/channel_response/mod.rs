//! What a caller sends back on the channels a provider opens.
//!
//! [`oci`] serves the image when it is the caller's to serve;
//! [`postgres`] is the far end of the plugin's database connection;
//! [`command`] is what running one produced.
//!
//! One module per kind of channel, each naming its own type `Frame`.
//! Nothing is re-exported upward: the module is the only thing telling
//! three types called `Frame` apart, so it has to stay in the path.
//!
//! [`oci`]'s frame is an ALIAS of
//! [`oci::response::Frame`](crate::shared::oci::response::Frame),
//! since relaying a registry answer is the same act wherever it
//! happens.
//!
//! All three are opaque bytes, for three different reasons: a registry
//! answer is HTTP and stays HTTP so that a caller can behave like a
//! proxy, a database connection must not be parsed, and a command's
//! output is not this specification's to describe.

pub mod command;
pub mod oci;
pub mod postgres;
