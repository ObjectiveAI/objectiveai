//! The requests over providers, served.
//!
//! The providers are records: [`add`], [`edit`], [`delete`],
//! [`enable`], [`disable`], [`tag`] and [`untag`] change them, each
//! change read into the [`ProviderBook`](crate::providers::ProviderBook)
//! and taking effect through [`reconcile`](crate::providers::reconcile);
//! [`get`] and [`list`] report them, each with where the daemon stands
//! with it now; [`discover`] lists the identities discoverable through
//! them.

pub mod add;
pub mod delete;
pub mod disable;
pub mod discover;
pub mod edit;
pub mod enable;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
