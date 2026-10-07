//! A list filter, read as a test of one record.
//!
//! The wire gives each kind one `Filter`, which its `list` request
//! narrows by and which a grant's `within` reaches by — the same type
//! with the same meaning, so one function serves both: a record
//! passes when it passes every member the filter gives, and a filter
//! with no member passes everything. A member that lists candidates
//! passes a record that is any one of them; `all_tags` a record that
//! carries every one; `any_tags` any one; an empty list is absent.
//! [`accounts`], [`roles`], [`providers_outgoing`],
//! [`providers_incoming`], [`agents_templates`], [`tools_templates`]
//! and [`resources`] are the tests.
//!
//! Evaluated here, in the daemon, against records loaded whole, and
//! not translated into the database's own queries: one truth for the
//! grant and the list, live state — whether a client is connected —
//! where a filter asks for it, and nothing to keep in step with the
//! SQL as kinds are added.

pub mod accounts;
pub mod agents;
pub mod agents_templates;
pub mod providers_incoming;
pub mod providers_outgoing;
pub mod resources;
pub mod roles;
pub mod routes;
pub mod tools;
pub mod tools_templates;
pub mod volumes;
