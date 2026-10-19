//! The roles a request names, resolved and judged.

use diverge_sdk::daemon::grant::roles::Over;
use sqlx::PgConnection;

use crate::judge::{self, Standing};
use crate::store::{self, RoleId, roles};

/// What naming roles came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedRoles {
    /// Every role named exists and the caller holds `grant` over each:
    /// their ids, in the order named, deduplicated.
    Roles(Vec<RoleId>),
    /// A role named is none the daemon has.
    NoRole,
    /// A role named is one the caller holds no `grant` grant over.
    Forbidden,
}

/// Resolve `names` to roles and judge the caller's `grant` over each.
/// A name given twice counts once. Existence is answered before
/// permission, as the wire orders `NoRole` before `Forbidden` for a
/// role the caller may not even name.
pub async fn named_roles(conn: &mut PgConnection, standing: &Standing, names: &[String]) -> Result<NamedRoles, store::Error> {
    let mut wanted: Vec<String> = Vec::new();
    for name in names {
        if !wanted.contains(name) {
            wanted.push(name.clone());
        }
    }
    let found = roles::by_names(conn, &wanted).await?;
    if found.len() != wanted.len() {
        return Ok(NamedRoles::NoRole);
    }
    if found.iter().any(|role| !judge::roles::over(standing, Over::Grant, role)) {
        return Ok(NamedRoles::Forbidden);
    }
    Ok(NamedRoles::Roles(found.iter().map(|role| role.id).collect()))
}
