//! The first account, seeded into a fresh database.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::grant::{self, Grant, Tagging, Within};
use sqlx::PgConnection;

use super::accounts;
use super::roles;
use super::Error;
use crate::judge::key;

/// The name of the root role and of the root account, and the root
/// account's credential identity and key, all four.
const ROOT: &str = "root";

/// Seed the root role and the root account.
///
/// The role `root` holds [`every_grant`]; the account `root`, named
/// and with a credential whose identity is `root`, holds the role,
/// and its key is the word `root` — a fresh daemon is open to whoever
/// reaches its port until that credential is rotated with an edit.
/// Both are ordinary records from then on: the account may be renamed,
/// given another key, or deleted, and the role edited or deleted once
/// nothing holds it. Root made itself, as far as the record says.
/// Called only when the database had no `accounts` table, so a daemon
/// whose root was deleted does not find it back at the next start.
pub async fn seed(conn: &mut PgConnection) -> Result<(), Error> {
    let creator = Creator::Client(Client {
        identity: ROOT.to_string(),
    });
    let role = match roles::create(
        conn,
        &roles::New {
            name: ROOT.to_string(),
            description: Some("every grant there is".to_string()),
            grants: every_grant(),
            creator: creator.clone(),
        },
    )
    .await?
    {
        roles::Created::Created(id) => id,
        // A fresh database holds no role; the table was made a
        // statement ago.
        roles::Created::Exists => return Ok(()),
    };
    let account = match accounts::create(
        conn,
        &accounts::New {
            name: Some(ROOT.to_string()),
            identity: Some(ROOT.to_string()),
            address: None,
            key_hash: Some(key::hash(ROOT)),
            description: Some("the first account".to_string()),
            creator,
        },
    )
    .await?
    {
        accounts::Created::Created(id) => id,
        accounts::Created::Exists => return Ok(()),
    };
    accounts::set_roles(conn, account, &[role]).await
}

/// Every grant of every kind: every making action, every action over
/// what exists reaching everything, both tagging actions over every
/// tag, and both actions over the database.
pub fn every_grant() -> Vec<Grant> {
    use grant::{
        accounts as ac, agents as ag, agents_templates as at, postgres as pg, providers_daemons as pd,
        providers_incoming as pi, providers_outgoing as po, roles as ro, tools as to,
        tools_templates as tt, volumes as vo,
    };
    let tagging = vec![Tagging::Tag, Tagging::Untag];
    vec![
        Grant::Agents(ag::Permission::Make(vec![ag::Make::Create])),
        Grant::Agents(ag::Permission::Over {
            actions: vec![
                ag::Over::Get,
                ag::Over::Delete,
                ag::Over::Edit,
                ag::Over::Message,
                ag::Over::Logs,
                ag::Over::List,
                ag::Over::Download,
                ag::Over::Upload,
                ag::Over::Transfer,
                ag::Over::Filetree,
            ],
            within: Within::Any,
        }),
        Grant::Agents(ag::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::AgentsTemplates(at::Permission::Make(vec![at::Make::Create])),
        Grant::AgentsTemplates(at::Permission::Over {
            actions: vec![at::Over::Get, at::Over::Delete, at::Over::List],
            within: Within::Any,
        }),
        Grant::AgentsTemplates(at::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::Tools(to::Permission::Make(vec![to::Make::Create, to::Make::Register])),
        Grant::Tools(to::Permission::Over {
            actions: vec![
                to::Over::Get,
                to::Over::Edit,
                to::Over::Attach,
                to::Over::Detach,
                to::Over::Delete,
                to::Over::List,
                to::Over::Download,
                to::Over::Upload,
                to::Over::Transfer,
                to::Over::Filetree,
                to::Over::Connect,
            ],
            within: Within::Any,
        }),
        Grant::Tools(to::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::ToolsTemplates(tt::Permission::Make(vec![tt::Make::Create])),
        Grant::ToolsTemplates(tt::Permission::Over {
            actions: vec![tt::Over::Get, tt::Over::Delete, tt::Over::List],
            within: Within::Any,
        }),
        Grant::ToolsTemplates(tt::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::ProvidersOutgoing(po::Permission::Make(vec![po::Make::Add])),
        Grant::ProvidersOutgoing(po::Permission::Over {
            actions: vec![po::Over::Get, po::Over::List, po::Over::Delete, po::Over::Edit],
            within: Within::Any,
        }),
        Grant::ProvidersOutgoing(po::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::ProvidersIncoming(pi::Permission::Make(vec![pi::Make::Add])),
        Grant::ProvidersIncoming(pi::Permission::Over {
            actions: vec![pi::Over::Get, pi::Over::List, pi::Over::Delete, pi::Over::Edit],
            within: Within::Any,
        }),
        Grant::ProvidersIncoming(pi::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::ProvidersDaemons(pd::Permission::Make(vec![pd::Make::Add])),
        Grant::ProvidersDaemons(pd::Permission::Over {
            actions: vec![pd::Over::Get, pd::Over::List, pd::Over::Delete, pd::Over::Edit],
            within: Within::Any,
        }),
        Grant::ProvidersDaemons(pd::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::Accounts(ac::Permission::Make(vec![ac::Make::Create])),
        Grant::Accounts(ac::Permission::Over {
            actions: vec![ac::Over::Get, ac::Over::List, ac::Over::Delete, ac::Over::Edit, ac::Over::Assign],
            within: Within::Any,
        }),
        Grant::Accounts(ac::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::Roles(ro::Permission::Make(vec![ro::Make::Create])),
        Grant::Roles(ro::Permission::Over {
            actions: vec![ro::Over::Get, ro::Over::List, ro::Over::Delete, ro::Over::Edit, ro::Over::Grant],
            within: Within::Any,
        }),
        Grant::Roles(ro::Permission::Tags {
            actions: tagging.clone(),
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::Volumes(vo::Permission::Make(vec![vo::Make::Create])),
        Grant::Volumes(vo::Permission::Over {
            actions: vec![
                vo::Over::Get,
                vo::Over::List,
                vo::Over::Delete,
                vo::Over::Edit,
                vo::Over::Stat,
                vo::Over::Download,
                vo::Over::Upload,
                vo::Over::Transfer,
                vo::Over::Mount,
                vo::Over::Filetree,
            ],
            within: Within::Any,
        }),
        Grant::Volumes(vo::Permission::Tags {
            actions: tagging,
            within: Within::Any,
            tags: Within::Any,
        }),
        Grant::Postgres(pg::Permission(vec![pg::Action::Get, pg::Action::List])),
    ]
}
