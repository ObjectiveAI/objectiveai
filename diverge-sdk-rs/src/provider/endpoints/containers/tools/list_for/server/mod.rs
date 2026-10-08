//! The server side of a listing: what a provider sends.
//!
//! [`response`] is what comes back on channel `0`: a container
//! added or removed, the word that the listing is whole, or the
//! error. And [`handle`] answers the request: the scope served
//! whole, from the request to the lister's stop and the finish.

pub mod response;

pub mod handle;
