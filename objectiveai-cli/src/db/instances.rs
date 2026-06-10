//! `instances` tier — aggregate enumeration backing
//! `agents instances list`.
//!
//! Lists agent instances that are descendants of a parent AIH and
//! reports per-agent stats joined across three tiers: `logs.messages`
//! (spawn/active timestamps + total logged), `message_queue` (active
//! queued count, resolving tag-targeted rows through `tags`), and
//! `tags` (the tag names bound to each agent).

use std::collections::HashMap;

use objectiveai_sdk::cli::command::agents::instances::list::ResponseItem;
use sqlx::Row as _;

use super::{Error, Pool};

/// List every agent instance whose AIH is a descendant of `parent`
/// (i.e. `agent_instance_hierarchy LIKE '{parent}/%'` — `parent`
/// itself is excluded), aggregating per-agent stats. Returns the SDK
/// `ResponseItem`s directly, sorted by AIH ascending.
pub async fn list_under_parent(
    pool: &Pool,
    parent: &str,
) -> Result<Vec<ResponseItem>, Error> {
    let like = format!("{parent}/%");

    // Per-agent aggregates: logs (spawned/active/logged) FULL OUTER
    // JOINed with active queue counts (queued). A queue row targeting a
    // tag resolves to that tag's BOUND AIH via `tags`; unbound/grouped
    // tags drop out (NULL eff_aih fails the LIKE). FULL OUTER JOIN
    // unions agents seen in either tier (a queue-only agent has NULL
    // log timestamps + logged 0).
    let rows = sqlx::query(
        "WITH log_agg AS ( \
             SELECT agent_instance_hierarchy AS aih, \
                    MIN(\"timestamp\") AS spawned, \
                    MAX(\"timestamp\") AS active, \
                    COUNT(*) AS logged \
             FROM logs.messages \
             WHERE agent_instance_hierarchy LIKE $1 \
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
             WHERE x.eff_aih LIKE $1 \
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
    .bind(&like)
    .fetch_all(&**pool)
    .await?;

    // Tag bindings for the whole subtree in one query, grouped per AIH
    // (newest-bound first), so we avoid a per-agent round trip.
    let tag_rows = sqlx::query(
        "SELECT agent_instance_hierarchy, name FROM tags \
         WHERE agent_instance_hierarchy LIKE $1 \
         ORDER BY updated_at DESC",
    )
    .bind(&like)
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
