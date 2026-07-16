//! Per-plugin/tool database compartments.
//!
//! A compartment is a postgres ROLE + same-named SCHEMA scoped to one
//! `(kind, owner, name, version)`. Inside its own schema the role has
//! full DDL/DML freedom (it owns it); through the shared
//! `objectiveai_read` group (provisioned by [`super::init`]) it has
//! READONLY access to the base `objectiveai` schema's tables; and it
//! has no privileges on any other compartment's schema. Postgres
//! caveat, accepted by design: other compartments' object NAMES
//! remain visible through `pg_catalog` — their data does not.
//!
//! [`ensure`] is idempotent and runs on every `tools run` /
//! `plugins run` spawn, returning the role-specific connection URL
//! the child receives as `OBJECTIVEAI_POSTGRES_URL`.
//!
//! The role's password is DERIVED — `xxh3_128(admin_password ":"
//! role_name)` — rather than randomized per spawn: N children of the
//! same compartment can spawn concurrently, and a rotate-on-spawn
//! scheme would invalidate URLs already handed to in-flight
//! siblings. Nothing is stored; if the admin password changes, the
//! next provision's `ALTER ROLE` re-derives in stride.

use super::{DbHandle, Error};

/// Which spawn surface owns the compartment. Folded into the
/// compartment name so a plugin and a tool with the same coordinates
/// never share a namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plugin,
    Tool,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Plugin => "plugin",
            Kind::Tool => "tool",
        }
    }
}

/// Serializes compartment provisioning across concurrent spawns —
/// the same one-lock-for-the-step pattern as the schema-apply lock
/// in [`super::init`].
const COMPARTMENT_LOCK_KEY: i64 = 0x0C0_3B_A87_AE_57_i64;

/// Ensure the compartment role + schema exist with the expected
/// grants, and return the child's connection URL.
pub async fn ensure(
    handle: &DbHandle,
    kind: Kind,
    owner: &str,
    name: &str,
    version: &str,
) -> Result<String, Error> {
    let role = compartment_name(kind, owner, name, version);
    let password = derived_password(&handle.admin_password, &role);
    // The password rides as a SQL string literal (role DDL takes no
    // bind parameters); it's hex from the derivation, but escape
    // defensively anyway.
    let password_literal = password.replace('\'', "''");
    // `role` is structurally `[a-z0-9_]` from the sanitizer, so
    // interpolating it as an identifier is injection-free. The
    // database name is arbitrary config input — quote it.
    let database_identifier = format!("\"{}\"", handle.database.replace('"', "\"\""));

    let mut tx = handle.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(COMPARTMENT_LOCK_KEY)
        .execute(&mut *tx)
        .await?;
    // 1. Role, created bare if absent (roles are cluster-wide; a
    //    sibling database may have provisioned it first)…
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {role}; EXCEPTION WHEN duplicate_object THEN NULL; END $$;",
    ))
    .execute(&mut *tx)
    .await?;
    // 2. …then unconditionally normalized: login with the derived
    //    password, explicitly INHERIT (the read grants arrive via
    //    group membership), and nothing else.
    sqlx::query(&format!(
        "ALTER ROLE {role} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE \
         PASSWORD '{password_literal}'",
    ))
    .execute(&mut *tx)
    .await?;
    // 3. The compartment schema, owned by the role — ownership IS
    //    the write grant.
    sqlx::query(&format!(
        "CREATE SCHEMA IF NOT EXISTS {role} AUTHORIZATION {role}",
    ))
    .execute(&mut *tx)
    .await?;
    // 4. Readonly over the base tables via the shared group.
    sqlx::query(&format!("GRANT objectiveai_read TO {role}"))
        .execute(&mut *tx)
        .await?;
    // 5. Own schema first on the search path: unqualified CREATEs
    //    land in the compartment (the role owns no other schema),
    //    unqualified SELECTs fall through to the base objectiveai
    //    tables.
    sqlx::query(&format!(
        "ALTER ROLE {role} IN DATABASE {database_identifier} \
         SET search_path = {role}, objectiveai",
    ))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(format!(
        "postgres://{role}:{password}@{address}/{database}",
        address = handle.address,
        database = percent_encoding::utf8_percent_encode(
            &handle.database,
            percent_encoding::NON_ALPHANUMERIC,
        ),
    ))
}

/// `{kind}_{owner}_{name}_{version}_{hash8}`, sanitized to
/// `[a-z0-9_]` with the readable part capped at 40 chars. The hash —
/// 8 hex chars of xxh3-64 over the RAW `kind/owner/name/version`
/// string — disambiguates coordinates the lossy sanitizer would
/// collide (`foo-bar` vs `foo_bar`) and must stay stable across
/// releases (a std hasher would not be). Always fits postgres's
/// 63-byte identifier cap and always starts with a letter (the
/// kind).
fn compartment_name(kind: Kind, owner: &str, name: &str, version: &str) -> String {
    let raw = format!("{}/{owner}/{name}/{version}", kind.as_str());
    let hash = twox_hash::XxHash3_64::oneshot(raw.as_bytes());
    let mut readable = format!(
        "{}_{}_{}_{}",
        kind.as_str(),
        sanitize(owner),
        sanitize(name),
        sanitize(version),
    );
    readable.truncate(40);
    format!("{readable}_{:08x}", hash as u32)
}

fn sanitize(part: &str) -> String {
    part.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Deterministic compartment password: 32 hex chars of
/// `xxh3_128(admin_password ":" role_name)`. Underivable without
/// the admin password (which already owns the whole database), and
/// stable so concurrent spawns of one compartment all mint the same
/// working URL.
fn derived_password(admin_password: &str, role: &str) -> String {
    let input = format!("{admin_password}:{role}");
    format!(
        "{:032x}",
        twox_hash::XxHash3_128::oneshot(input.as_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compartment_names_are_postgres_safe_and_distinct() {
        let a = compartment_name(Kind::Plugin, "testorg", "test-mcp-plugin", "1.0.0");
        let b = compartment_name(Kind::Plugin, "testorg", "test_mcp_plugin", "1.0.0");
        let c = compartment_name(Kind::Tool, "testorg", "test-mcp-plugin", "1.0.0");
        for n in [&a, &b, &c] {
            assert!(n.len() <= 63, "{n} exceeds identifier cap");
            assert!(
                n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{n} carries non-identifier chars",
            );
            assert!(n.starts_with("plugin_") || n.starts_with("tool_"));
        }
        // Sanitization collides the readable part; the hash must not.
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn long_coordinates_stay_under_the_identifier_cap() {
        let n = compartment_name(
            Kind::Plugin,
            "an-extremely-long-github-organization-name",
            "a-repository-name-that-goes-on-and-on-forever",
            "10.20.30-beta.4",
        );
        assert!(n.len() <= 63, "{n} ({}) exceeds identifier cap", n.len());
    }
}
