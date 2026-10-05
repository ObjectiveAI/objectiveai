//! Roles: named lists of grants, held by accounts.
//!
//! A ROLE is a name and a list of [grants](crate::daemon::grant), each
//! a permission over one kind of thing the daemon holds. An
//! [account](crate::daemon::endpoints::accounts) holds zero or more
//! roles, and may do what any grant of any role it holds allows: the
//! union, with nothing that denies, so the order of roles and of grants
//! never matters, and an account with no role may do nothing. A role
//! holds no roles. One role per name, and the name is what every
//! request names the role by; naming a role in an account's `roles`, at
//! the account's create or edit, takes the `grant` grant over the role.
//!
//! [`create`] makes one; [`get`] answers one as a list would; [`list`]
//! lists them, narrowed; [`delete`] removes one no account holds;
//! [`edit`] changes its description or replaces its grants whole, which
//! every account holding it is judged by from then on; [`tag`] and
//! [`untag`] change its tags. What a list and a get report is a
//! [`Role`](list::server::response::Role): the role with its grants,
//! and the accounts that hold it.

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
