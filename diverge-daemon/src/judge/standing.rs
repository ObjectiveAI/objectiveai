//! The account as of one request.

use diverge_sdk::daemon::grant::{self, Grant};
use sqlx::PgConnection;

use super::Who;
use crate::store;

/// Who is asking, for the length of one request: the identity they
/// are served under, and every grant they hold. An account's is every
/// grant of every role it holds, read inside the request's
/// transaction so that the judgment and the write see one state;
/// `None` is an account deleted since its client was admitted, which
/// every handler answers as `Forbidden`. A dependency tool's is its
/// template's grants, fixed when it was deployed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// The account's name when it has one, else its credential's
    /// identity: what a `Creator::Client` carries.
    pub identity: String,
    /// Every grant, in no order.
    grants: Vec<Grant>,
}

impl Standing {
    /// The standing of `who` now: an account's read fresh, a
    /// dependency's as it was fixed.
    pub async fn of(conn: &mut PgConnection, who: Who) -> Result<Option<Standing>, store::Error> {
        match who {
            Who::Account(id) => Ok(store::of_account(conn, id)
                .await?
                .map(|(identity, grants)| Standing { identity, grants })),
            Who::Dependency(standing) => Ok(Some((*standing).clone())),
        }
    }

    /// A standing from grants alone: a dependency tool's, from its
    /// template.
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

    /// The grants over daemon records.
    pub fn providers_daemons(&self) -> impl Iterator<Item = &grant::providers_daemons::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersDaemons(permission) => Some(permission),
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

    /// The grants over the database.
    pub fn postgres(&self) -> impl Iterator<Item = &grant::postgres::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Postgres(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over volumes.
    pub fn volumes(&self) -> impl Iterator<Item = &grant::volumes::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Volumes(permission) => Some(permission),
            _ => None,
        })
    }
}
