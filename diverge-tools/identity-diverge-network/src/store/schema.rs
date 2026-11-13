//! The table, made where absent.

use sqlx::PgPool;

/// Every statement that makes the tool's table, in order. Each is `if
/// not exists`, so ensuring them again is nothing.
///
/// A row is one account: who it is — its issuer and its id there,
/// under the template the tool was made from — and what its issuer
/// answered its login, and every renewal since, with. The four token
/// columns are the SDK's
/// [`Tokens`](diverge_sdk::identity_diverge_network::Tokens), member
/// for member.
const STATEMENTS: [&str; 1] = ["create table if not exists identity_accounts (
        template text not null,
        issuer text not null,
        id text not null,
        access_token text,
        access_expires bigint,
        refresh_token text,
        id_token text,
        added timestamptz not null default now(),
        primary key (template, issuer, id)
    )"];

/// Make the table where absent.
pub async fn ensure(pool: &PgPool) -> Result<(), sqlx::Error> {
    for statement in STATEMENTS {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
