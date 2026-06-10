//! `instances` tier — aggregate enumeration backing
//! `agents instances {list, get}`.
//!
//! Reports per-agent stats joined across three tiers: `logs.messages`
//! (spawn/active timestamps + total logged), `message_queue` (active
//! queued count, resolving tag-targeted rows through `tags`), and
//! `tags` (the tag names bound to each agent).
//!
//! Two entry points share one aggregate, differing only in which
//! agents they scope to:
//! - [`list_under_parent`] — the DIRECT children of a parent AIH.
//! - [`get_exact`] — one exact agent AIH.

use std::collections::HashMap;

use objectiveai_sdk::cli::command::agents::instances::list::ResponseItem;
use sqlx::Row as _;

use super::{Error, Pool};

/// Aggregate per-agent stats for the agents selected by the AIH
/// predicate. Exactly one of `exact` / `prefix` is `Some`:
/// - `exact = Some(aih)` → the single agent whose AIH equals `aih`.
/// - `prefix = Some("{p}/%")` + `prefix_deep = Some("{p}/%/%")` → the
///   direct children of `p` (starts with `{p}/`, no further `/`).
///
/// The same 3-bind predicate gates `log_agg`, `queue_agg`, and the
/// tags subtree query. Returns SDK `ResponseItem`s sorted by AIH.
async fn aggregate(
    pool: &Pool,
    exact: Option<&str>,
    prefix: Option<&str>,
    prefix_deep: Option<&str>,
) -> Result<Vec<ResponseItem>, Error> {
    // logs (spawned/active/logged) FULL OUTER JOINed with active queue
    // counts (queued). A queue row targeting a tag resolves to that
    // tag's BOUND AIH via `tags`; unbound/grouped tags drop out (NULL
    // eff_aih fails the predicate). FULL OUTER JOIN unions agents seen
    // in either tier.
    let rows = sqlx::query(
        "WITH log_agg AS ( \
             SELECT agent_instance_hierarchy AS aih, \
                    MIN(\"timestamp\") AS spawned, \
                    MAX(\"timestamp\") AS active, \
                    COUNT(*) AS logged \
             FROM logs.messages \
             WHERE ( \
                 ($1::text IS NOT NULL AND agent_instance_hierarchy = $1) \
                 OR ($2::text IS NOT NULL \
                     AND agent_instance_hierarchy LIKE $2 \
                     AND agent_instance_hierarchy NOT LIKE $3) \
             ) \
             GROUP BY agent_instance_hierarchy \
         ), \
         queue_agg AS ( \
             SELECT eff_aih AS aih, COUNT(*) AS queued \
             FROM ( \
                 SELECT COALESCE( \
                            mq.agent_instance_hierarchy, \
                            t.agent_instance_hierarchy \
                        ) AS eff_aih \
                 FROM message_queue mq \
                 LEFT JOIN tags t \
                     ON mq.agent_tag IS NOT NULL AND t.name = mq.agent_tag \
                 WHERE mq.active = TRUE \
             ) x \
             WHERE ( \
                 ($1::text IS NOT NULL AND x.eff_aih = $1) \
                 OR ($2::text IS NOT NULL \
                     AND x.eff_aih LIKE $2 \
                     AND x.eff_aih NOT LIKE $3) \
             ) \
             GROUP BY eff_aih \
         ) \
         SELECT COALESCE(l.aih, q.aih)   AS aih, \
                l.spawned                AS spawned, \
                l.active                 AS active, \
                COALESCE(l.logged, 0)::bigint AS logged, \
                COALESCE(q.queued, 0)::bigint AS queued \
         FROM log_agg l \
         FULL OUTER JOIN queue_agg q ON q.aih = l.aih \
         ORDER BY aih ASC",
    )
    .bind(exact)
    .bind(prefix)
    .bind(prefix_deep)
    .fetch_all(&**pool)
    .await?;

    // Tag bindings for the same scope in one query, grouped per AIH
    // (newest-bound first), so we avoid a per-agent round trip.
    let tag_rows = sqlx::query(
        "SELECT agent_instance_hierarchy, name FROM tags \
         WHERE ( \
             ($1::text IS NOT NULL AND agent_instance_hierarchy = $1) \
             OR ($2::text IS NOT NULL \
                 AND agent_instance_hierarchy LIKE $2 \
                 AND agent_instance_hierarchy NOT LIKE $3) \
         ) \
         ORDER BY updated_at DESC",
    )
    .bind(exact)
    .bind(prefix)
    .bind(prefix_deep)
    .fetch_all(&**pool)
    .await?;
    let mut tags_by_aih: HashMap<String, Vec<String>> = HashMap::new();
    for row in tag_rows {
        let aih: String = row.try_get("agent_instance_hierarchy")?;
        let name: String = row.try_get("name")?;
        tags_by_aih.entry(aih).or_default().push(name);
    }

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let aih: String = row.try_get("aih")?;
        let spawned: Option<i64> = row.try_get("spawned")?;
        let active: Option<i64> = row.try_get("active")?;
        let logged: i64 = row.try_get("logged")?;
        let queued: i64 = row.try_get("queued")?;
        let tags = tags_by_aih.remove(&aih).unwrap_or_default();
        out.push(ResponseItem {
            agent_instance_hierarchy: aih,
            tags,
            queued: queued as u64,
            timestamp_spawned: spawned,
            timestamp_active: active,
            logged: logged as u64,
        });
    }
    Ok(out)
}

/// List the DIRECT children of `parent` (AIH `LIKE '{parent}/%'` with
/// no further `/` — `parent` itself and deeper descendants excluded),
/// with per-agent aggregates. Sorted by AIH ascending.
pub async fn list_under_parent(
    pool: &Pool,
    parent: &str,
) -> Result<Vec<ResponseItem>, Error> {
    let prefix = format!("{parent}/%");
    let prefix_deep = format!("{parent}/%/%");
    aggregate(pool, None, Some(&prefix), Some(&prefix_deep)).await
}

/// Aggregate stats for one EXACT agent AIH. Always returns an item:
/// when the agent has no logs/queue activity it is zero-filled (its
/// AIH + bound tags + zero counts + null timestamps), so an
/// explicitly-named target always yields a row.
pub async fn get_exact(pool: &Pool, aih: &str) -> Result<ResponseItem, Error> {
    let mut items = aggregate(pool, Some(aih), None, None).await?;
    if let Some(item) = items.pop() {
        return Ok(item);
    }
    // No logs/queue rows for this agent; still surface it with its
    // tags (which may exist independently) and zeroed activity.
    let tags = super::tags::tags_for_hierarchy(pool, aih).await?;
    Ok(ResponseItem {
        agent_instance_hierarchy: aih.to_string(),
        tags,
        queued: 0,
        timestamp_spawned: None,
        timestamp_active: None,
        logged: 0,
    })
}
