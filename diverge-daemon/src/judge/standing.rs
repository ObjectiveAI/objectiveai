//! The account as of one request.

use diverge_sdk::daemon::grant::{self, Grant};
use sqlx::PgConnection;

use super::Who;
use crate::store;

/// What an account is, for the length of one request: the identity
/// it is served under, and every grant of every role it holds. Read
/// inside the request's transaction so that the judgment and the
/// write see one state; `None` is an account deleted since its client
/// was admitted, which every handler answers as `Forbidden`.
#[derive(Debug, Clone)]
pub struct Standing {
    /// The account's name when it has one, else its credential's
    /// identity: what a `Creator::Client` carries.
    pub identity: String,
    /// Every grant, in no order.
    grants: Vec<Grant>,
}

impl Standing {
    /// The standing of `who` now.
    pub async fn of(conn: &mut PgConnection, who: Who) -> Result<Option<Standing>, store::Error> {
        Ok(store::of_account(conn, who.id)
            .await?
            .map(|(identity, grants)| Standing { identity, grants }))
    }

    /// A standing from grants alone, for judging without a store.
    pub fn from_grants(identity: String, grants: Vec<Grant>) -> Standing {
        Standing { identity, grants }
    }

    /// The grants over accounts.
    pub fn accounts(&self) -> impl Iterator<Item = &grant::accounts::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Accounts(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over roles.
    pub fn roles(&self) -> impl Iterator<Item = &grant::roles::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Roles(permission) => Some(permission),
            _ => None,
        })
    }
}
