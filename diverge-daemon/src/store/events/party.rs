//! One end of an edge, as the store holds it.

use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::events::Party;
use diverge_sdk::daemon::key::Owner;
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// One end's columns, as an events row holds them: its kind, and the
/// identity or the one row that kind names, the rest absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns {
    /// `account`, `agent`, `standalone_tool` or `dependency_tool`.
    pub kind: &'static str,
    /// An account's issuer.
    pub issuer: Option<String>,
    /// An account's subject.
    pub subject: Option<String>,
    /// An agent's row.
    pub agent: Option<i64>,
    /// A standalone tool's row.
    pub tool: Option<i64>,
    /// A dependency tool's row.
    pub dependency: Option<i64>,
}

impl Columns {
    /// Nothing set, of `kind`: what each variant fills one member of.
    fn of(kind: &'static str) -> Columns {
        Columns { kind, issuer: None, subject: None, agent: None, tool: None, dependency: None }
    }
}

/// The columns the seam's `creator` is, by the rows it names: the
/// client by its identity; an agent by its template and its index; a
/// tool made from a template by the same; a dependency tool by its
/// template and its owner, an owning agent read by its template and
/// its index. `None` when the row is gone — deleted between the
/// exchange and its recording — so there is no edge to record.
pub async fn resolve(conn: &mut PgConnection, creator: &Creator) -> Result<Option<Columns>, Error> {
    let found: Option<i64> = match creator {
        Creator::Client(client) => {
            return Ok(Some(Columns {
                issuer: Some(client.identity.issuer.clone()),
                subject: Some(client.identity.subject.clone()),
                ..Columns::of("account")
            }));
        }
        Creator::Agent(agent) => {
            let found = sqlx::query_scalar("SELECT id FROM diverge.agents WHERE template = $1 AND index = $2")
                .bind(&agent.template)
                .bind(i64::try_from(agent.index).unwrap_or(i64::MAX))
                .fetch_optional(&mut *conn)
                .await?;
            return Ok(found.map(|id| Columns { agent: Some(id), ..Columns::of("agent") }));
        }
        Creator::Tool(creator::Tool::Record { template, index, .. }) => {
            let found = sqlx::query_scalar("SELECT id FROM diverge.tools WHERE kind = 'created' AND template = $1 AND index = $2")
                .bind(template)
                .bind(i64::try_from(*index).unwrap_or(i64::MAX))
                .fetch_optional(&mut *conn)
                .await?;
            return Ok(found.map(|id| Columns { tool: Some(id), ..Columns::of("standalone_tool") }));
        }
        Creator::Tool(creator::Tool::Dependency { template, owner: Owner::Agent(agent) }) => {
            sqlx::query_scalar(
                "SELECT t.id FROM diverge.dependency_tools t JOIN diverge.agents a ON a.id = t.owner_agent \
                 WHERE t.template = $1 AND t.scope = 'instance' AND a.template = $2 AND a.index = $3",
            )
            .bind(template)
            .bind(&agent.template)
            .bind(i64::try_from(agent.index).unwrap_or(i64::MAX))
            .fetch_optional(&mut *conn)
            .await?
        }
        Creator::Tool(creator::Tool::Dependency { template, owner: Owner::AgentTemplate { template: owner } }) => {
            sqlx::query_scalar("SELECT id FROM diverge.dependency_tools WHERE template = $1 AND scope = 'template' AND owner_template = $2")
                .bind(template)
                .bind(owner)
                .fetch_optional(&mut *conn)
                .await?
        }
        Creator::Tool(creator::Tool::Dependency { template, owner: Owner::Global }) => {
            sqlx::query_scalar("SELECT id FROM diverge.dependency_tools WHERE template = $1 AND scope = 'global'")
                .bind(template)
                .fetch_optional(&mut *conn)
                .await?
        }
    };
    Ok(found.map(|id| Columns { dependency: Some(id), ..Columns::of("dependency_tool") }))
}

/// The party one end of a row of [`all`](super::all) is, `side` being
/// `sender` or `receiver`: its kind, and the identity or the template
/// and index of the row it names, joined in as `<side>_template` and
/// `<side>_index`.
pub(super) fn party(row: &PgRow, side: &str) -> Result<Party, Error> {
    let kind: String = row.try_get(format!("{side}_kind").as_str())?;
    if kind == "account" {
        return Ok(Party::Account {
            issuer: row.try_get(format!("{side}_issuer").as_str())?,
            subject: row.try_get(format!("{side}_subject").as_str())?,
        });
    }
    let template: String = row.try_get(format!("{side}_template").as_str())?;
    let index: i64 = row.try_get(format!("{side}_index").as_str())?;
    let index = u64::try_from(index).unwrap_or(0);
    match kind.as_str() {
        "agent" => Ok(Party::Agent { template, index }),
        "standalone_tool" => Ok(Party::StandaloneTool { template, index }),
        "dependency_tool" => Ok(Party::DependencyTool { template, index }),
        _ => Err(Error::Json(serde::de::Error::custom(format!("`{kind}` is not an event party's kind")))),
    }
}
