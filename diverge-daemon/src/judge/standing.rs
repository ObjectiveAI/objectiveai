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

    /// The grants over outgoing providers.
    pub fn providers_outgoing(&self) -> impl Iterator<Item = &grant::providers_outgoing::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersOutgoing(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over agent templates.
    pub fn agents_templates(&self) -> impl Iterator<Item = &grant::agents_templates::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::AgentsTemplates(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over tool templates.
    pub fn tools_templates(&self) -> impl Iterator<Item = &grant::tools_templates::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ToolsTemplates(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over resources.
    pub fn resources(&self) -> impl Iterator<Item = &grant::resources::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Resources(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over incoming credentials.
    pub fn providers_incoming(&self) -> impl Iterator<Item = &grant::providers_incoming::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersIncoming(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over agents.
    pub fn agents(&self) -> impl Iterator<Item = &grant::agents::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Agents(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over tools.
    pub fn tools(&self) -> impl Iterator<Item = &grant::tools::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Tools(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over routes.
    pub fn routes(&self) -> impl Iterator<Item = &grant::routes::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Routes(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over the database.
    pub fn postgres(&self) -> impl Iterator<Item = &grant::postgres::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Postgres(permission) => Some(permission),
            _ => None,
        })
    }
}
