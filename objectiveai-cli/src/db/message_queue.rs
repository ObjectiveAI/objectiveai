//! Deferred-message storage for `agents queue {add, list, read id}`.
//!
//! Mirrors the sqlite predecessor's split: a `message_queue` row carries
//! either an `agent_instance_hierarchy` or an `agent_tag` (CHECK
//! enforces); the master `message_queue_contents` registry has FK-cascade
//! chains down to per-kind tables (`message_queue_texts`, `message_queue_images`,
//! `message_queue_audios`, `message_queue_videos`, `message_queue_files`) — one DELETE on
//! `message_queue` sweeps every per-kind row through the cascades.

use objectiveai_sdk::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContent, RichContentPart, VideoUrl,
};
use objectiveai_sdk::cli::command::agents::logs::read::all::ResponseContent;
use objectiveai_sdk::cli::command::agents::queue::read::pending::{
    LookupState, ResponseItem,
};
use sqlx::{PgConnection, Postgres, Row as _, Transaction};

use super::{Error, Pool};

/// One content row — typed payload of a single `message_queue_contents.id`.
#[derive(Debug, Clone)]
pub enum ContentRow {
    Text(String),
    Image(ImageUrl),
    Audio(InputAudio),
    Video(VideoUrl),
    File(File),
}

/// Map one [`ContentRow`] to its matching SDK [`RichContentPart`] variant.
pub fn content_row_to_part(row: ContentRow) -> RichContentPart {
    match row {
        ContentRow::Text(text) => RichContentPart::Text { text },
        ContentRow::Image(image_url) => RichContentPart::ImageUrl { image_url },
        ContentRow::Audio(input_audio) => RichContentPart::InputAudio { input_audio },
        ContentRow::Video(video_url) => RichContentPart::VideoUrl { video_url },
        ContentRow::File(file) => RichContentPart::File { file },
    }
}

/// One drained message — carries enough metadata to re-INSERT the
/// original row.
#[derive(Debug, Clone)]
pub struct DrainedMessage {
    pub agent_instance_hierarchy: Option<String>,
    pub agent_tag: Option<String>,
    pub key: Option<String>,
    pub enqueued_at: i64,
    pub content: RichContent,
}

/// One addressed delivery target.
#[derive(Debug, Clone)]
pub struct DeliveryTarget {
    pub agent_instance_hierarchy: String,
    pub agent_tag: Option<String>,
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// read_content — single-id resolver
// ---------------------------------------------------------------------------

/// Look up a single content row by `message_queue_contents.id`. Returns
/// `None` when the id doesn't exist; a per-kind miss is DB corruption
/// and surfaces as `Error::InvalidData`.
pub async fn read_content(
    pool: &Pool,
    id: i64,
) -> Result<Option<ContentRow>, Error> {
    let mut conn = pool.acquire().await?;
    read_content_on_conn(&mut conn, id).await
}

/// `read_content` that operates on an externally-held `&mut PgConnection`
/// (or the `&mut **tx` deref of a `Transaction<'_, Postgres>`). Lets
/// the drain helpers reconstruct content inside their surrounding
/// transaction. We re-borrow `&mut *conn` for each query so the
/// per-kind probe + per-payload fetch don't try to move `conn`.
async fn read_content_on_conn(
    conn: &mut PgConnection,
    id: i64,
) -> Result<Option<ContentRow>, Error> {
    let row = sqlx::query("SELECT kind FROM message_queue_contents WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let kind: String = row.try_get(0)?;
    let result = match kind.as_str() {
        "text" => {
            let r = sqlx::query("SELECT text FROM message_queue_texts WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await?;
            ContentRow::Text(r.try_get(0)?)
        }
        "image" => {
            let r = sqlx::query("SELECT url, detail FROM message_queue_images WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await?;
            let url: String = r.try_get(0)?;
            let detail_str: Option<String> = r.try_get(1)?;
            let detail = match detail_str {
                Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
                None => None,
            };
            ContentRow::Image(ImageUrl { url, detail })
        }
        "audio" => {
            let r = sqlx::query("SELECT data, format FROM message_queue_audios WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await?;
            ContentRow::Audio(InputAudio {
                data: r.try_get(0)?,
                format: r.try_get(1)?,
            })
        }
        "video" => {
            let r = sqlx::query("SELECT url FROM message_queue_videos WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await?;
            ContentRow::Video(VideoUrl { url: r.try_get(0)? })
        }
        "file" => {
            let r = sqlx::query(
                "SELECT file_data, file_id, filename, file_url \
                 FROM message_queue_files WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&mut *conn)
            .await?;
            ContentRow::File(File {
                file_data: r.try_get(0)?,
                file_id: r.try_get(1)?,
                filename: r.try_get(2)?,
                file_url: r.try_get(3)?,
            })
        }
        other => {
            return Err(Error::InvalidData(format!(
                "unknown message_queue_contents.kind: {other}"
            )));
        }
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// enqueue_with_content — INSERT message_queue + walk content into per-kind tables
// ---------------------------------------------------------------------------

/// Atomic enqueue: inserts the `message_queue` row, walks `content` and
/// extracts every part into a per-kind table referenced by id, then
/// UPDATEs the `message_queue.content` column with the assembled
/// [`ResponseContent`] JSON (`One(i64)` for single-part, `Many(Vec<i64>)`
/// for multi-part). Returns the new `message_queue.id`. Everything runs
/// inside one transaction — failure rolls every content row back.
pub async fn enqueue_with_content(
    pool: &Pool,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    key: Option<String>,
    content: RichContent,
) -> Result<i64, Error> {
    let mut tx = pool.begin().await?;
    let message_queue_id = enqueue_with_content_in_tx(
        &mut tx,
        agent_instance_hierarchy.as_deref(),
        agent_tag.as_deref(),
        key.as_deref(),
        now_seconds(),
        content,
    )
    .await?;
    tx.commit().await?;
    Ok(message_queue_id)
}

/// Insert one message row + its content rows inside an existing
/// transaction. `enqueued_at` is parameterised so callers that
/// preserve the original FIFO timestamp (e.g. batched bulk inserts)
/// can pass it through unchanged.
async fn enqueue_with_content_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    key: Option<&str>,
    enqueued_at: i64,
    content: RichContent,
) -> Result<i64, Error> {
    if let Some(key_value) = key {
        // Upsert: drop any prior row for this (target, key) pair so
        // the partial unique index never trips. Cascade on
        // `message_queue_contents.message_queue_id` sweeps the prior row's content
        // rows in the same transaction.
        sqlx::query(
            "DELETE FROM message_queue \
             WHERE key = $3 \
               AND ( \
                   (agent_instance_hierarchy IS NOT NULL \
                    AND $1::text IS NOT NULL \
                    AND agent_instance_hierarchy = $1) \
                   OR \
                   (agent_tag IS NOT NULL \
                    AND $2::text IS NOT NULL \
                    AND agent_tag = $2) \
               )",
        )
        .bind(agent_instance_hierarchy)
        .bind(agent_tag)
        .bind(key_value)
        .execute(&mut **tx)
        .await?;
    }
    // Empty `message` placeholder — overwritten by the final UPDATE
    // once we know the id-referenced shape. `message_queue.content` is NOT
    // NULL so we need *some* value here.
    let message_queue_id: i64 = sqlx::query_scalar(
        "INSERT INTO message_queue (agent_instance_hierarchy, agent_tag, content, enqueued_at, key) \
         VALUES ($1, $2, '', $3, $4) \
         RETURNING id",
    )
    .bind(agent_instance_hierarchy)
    .bind(agent_tag)
    .bind(enqueued_at)
    .bind(key)
    .fetch_one(&mut **tx)
    .await?;
    let response_content = walk_rich(tx, message_queue_id, content).await?;
    let json = serde_json::to_string(&response_content)?;
    sqlx::query("UPDATE message_queue SET content = $1 WHERE id = $2")
        .bind(json)
        .bind(message_queue_id)
        .execute(&mut **tx)
        .await?;
    Ok(message_queue_id)
}

async fn walk_rich(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    content: RichContent,
) -> Result<ResponseContent, Error> {
    match content {
        RichContent::Text(text) => {
            let id = insert_content_text(tx, message_queue_id, &text).await?;
            Ok(ResponseContent::One(id))
        }
        RichContent::Parts(parts) => {
            let mut ids = Vec::with_capacity(parts.len());
            for part in parts {
                ids.push(insert_content_part(tx, message_queue_id, part).await?);
            }
            if ids.len() == 1 {
                Ok(ResponseContent::One(ids.remove(0)))
            } else {
                Ok(ResponseContent::Many(ids))
            }
        }
    }
}

async fn insert_content_part(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    part: RichContentPart,
) -> Result<i64, Error> {
    match part {
        RichContentPart::Text { text } => insert_content_text(tx, message_queue_id, &text).await,
        RichContentPart::ImageUrl { image_url } => {
            insert_content_image(tx, message_queue_id, &image_url).await
        }
        RichContentPart::InputAudio { input_audio } => {
            insert_content_audio(tx, message_queue_id, &input_audio).await
        }
        RichContentPart::InputVideo { video_url }
        | RichContentPart::VideoUrl { video_url } => {
            insert_content_video(tx, message_queue_id, &video_url).await
        }
        RichContentPart::File { file } => insert_content_file(tx, message_queue_id, &file).await,
    }
}

async fn mint_content_id(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    kind: &str,
) -> Result<i64, Error> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO message_queue_contents (message_queue_id, kind) VALUES ($1, $2) RETURNING id",
    )
    .bind(message_queue_id)
    .bind(kind)
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

async fn insert_content_text(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    text: &str,
) -> Result<i64, Error> {
    let id = mint_content_id(tx, message_queue_id, "text").await?;
    sqlx::query("INSERT INTO message_queue_texts (id, text) VALUES ($1, $2)")
        .bind(id)
        .bind(text)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

async fn insert_content_image(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    image: &ImageUrl,
) -> Result<i64, Error> {
    let id = mint_content_id(tx, message_queue_id, "image").await?;
    let detail = image
        .detail
        .as_ref()
        .map(|d| serde_json::to_value(d).map(|v| v.as_str().map(str::to_string)))
        .transpose()?
        .flatten();
    sqlx::query("INSERT INTO message_queue_images (id, url, detail) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&image.url)
        .bind(detail)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

async fn insert_content_audio(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    audio: &InputAudio,
) -> Result<i64, Error> {
    let id = mint_content_id(tx, message_queue_id, "audio").await?;
    sqlx::query("INSERT INTO message_queue_audios (id, data, format) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&audio.data)
        .bind(&audio.format)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

async fn insert_content_video(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    video: &VideoUrl,
) -> Result<i64, Error> {
    let id = mint_content_id(tx, message_queue_id, "video").await?;
    sqlx::query("INSERT INTO message_queue_videos (id, url) VALUES ($1, $2)")
        .bind(id)
        .bind(&video.url)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

async fn insert_content_file(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    file: &File,
) -> Result<i64, Error> {
    let id = mint_content_id(tx, message_queue_id, "file").await?;
    sqlx::query(
        "INSERT INTO message_queue_files (id, file_data, file_id, filename, file_url) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(&file.file_data)
    .bind(&file.file_id)
    .bind(&file.filename)
    .bind(&file.file_url)
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// List — JOIN message_queue ⨝ tags with three-rule predicate (Direct child /
// BOUND tag / PENDING tag).
// ---------------------------------------------------------------------------

/// List all queued message_queue visible under `parent`. Three rules:
/// 1. Direct row: `agent_instance_hierarchy` is a direct child of
///    `parent` (LIKE `parent/%` AND no further `/`).
/// 2. BOUND tag (LEFT JOIN matches): tag's bound hierarchy is a direct
///    child of `parent`.
/// 3. PENDING tag: tag's stored parent equals `parent` exactly.
pub async fn list(pool: &Pool, parent: &str) -> Result<Vec<ResponseItem>, Error> {
    let prefix_len = parent.len() as i32;
    let pattern = format!("{parent}/%");
    let rows = sqlx::query(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.content, \
                t.agent_instance_hierarchy        AS tag_bound_hierarchy, \
                g.id                              AS tag_group_id, \
                g.agent_spec                      AS tag_group_spec, \
                g.parent_agent_instance_hierarchy AS tag_group_parent \
         FROM message_queue p \
         LEFT JOIN tags t ON p.agent_tag = t.name \
         LEFT JOIN tag_groups g ON g.id = t.tag_group \
         WHERE \
                /* Direct row: agent_instance_hierarchy is a direct child of $1 */ \
                ( \
                    p.agent_instance_hierarchy IS NOT NULL \
                    AND p.agent_instance_hierarchy LIKE $2 \
                    AND position('/' in substring(p.agent_instance_hierarchy from cast($3 as int) + 2)) = 0 \
                ) \
             /* BOUND tag: tag's bound hierarchy is a direct child of $1 */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND t.agent_instance_hierarchy IS NOT NULL \
                    AND t.agent_instance_hierarchy LIKE $2 \
                    AND position('/' in substring(t.agent_instance_hierarchy from cast($3 as int) + 2)) = 0 \
                ) \
             /* GROUPED tag: group's stored parent equals $1 exactly */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND g.parent_agent_instance_hierarchy = $1 \
                ) \
         ORDER BY p.id",
    )
    .bind(parent)
    .bind(&pattern)
    .bind(prefix_len)
    .fetch_all(&**pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get(0)?;
        let agent_instance_hierarchy: Option<String> = row.try_get(1)?;
        let agent_tag: Option<String> = row.try_get(2)?;
        let key: Option<String> = row.try_get(3)?;
        let content_json: String = row.try_get(4)?;
        let tag_bound_hierarchy: Option<String> = row.try_get(5)?;
        let tag_group_id: Option<i64> = row.try_get(6)?;
        let tag_group_spec: Option<serde_json::Value> = row.try_get(7)?;
        let tag_group_parent: Option<String> = row.try_get(8)?;

        let content: ResponseContent = if content_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&content_json)?
        };
        if let Some(h) = agent_instance_hierarchy {
            let agent_instance = h
                .strip_prefix(&format!("{parent}/"))
                .map(str::to_string)
                .unwrap_or_else(|| super::tags::leaf_of(&h).to_string());
            out.push(ResponseItem::AgentInstance {
                id,
                agent_instance,
                key,
                content,
            });
        } else if let Some(tag) = agent_tag {
            let Some(state) = (match (
                tag_bound_hierarchy,
                tag_group_id,
                tag_group_spec,
                tag_group_parent,
            ) {
                (Some(agent_instance_hierarchy), None, None, None) => {
                    Some(LookupState::Bound { agent_instance_hierarchy })
                }
                (None, Some(group_id), Some(spec_value), Some(group_parent)) => {
                    let agent_spec = serde_json::from_value(spec_value)?;
                    Some(LookupState::Grouped {
                        tag_group_id: group_id,
                        agent_spec,
                        parent_agent_instance_hierarchy: group_parent,
                    })
                }
                _ => None,
            }) else {
                continue;
            };
            out.push(ResponseItem::Tag {
                id,
                agent_tag: tag,
                state,
                key,
                content,
            });
        } else {
            // CHECK guarantees unreachable; silently skip malformed.
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Drain — atomically pull rows + reconstruct each as a RichContent.
// ---------------------------------------------------------------------------

/// Row data the drain SELECT pulls out of `message_queue`.
struct DrainedRow {
    message_queue_id: i64,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    key: Option<String>,
    enqueued_at: i64,
    content_json: String,
}

/// Drain rows targeting `target_hierarchy`, `target_tag` (if some), or
/// any BOUND tag whose `agent_instance_hierarchy` equals
/// `target_hierarchy`. Returns drained message_queue oldest-first; deletes
/// the matched rows in the same transaction.
pub async fn drain_for_message(
    pool: &Pool,
    target_hierarchy: &str,
    target_tag: Option<&str>,
) -> Result<Vec<DrainedMessage>, Error> {
    let mut tx = pool.begin().await?;
    let rows = collect_matching_for_message(
        &mut tx,
        target_hierarchy,
        target_tag,
    )
    .await?;
    let drained = reconstruct_and_delete(&mut tx, rows).await?;
    tx.commit().await?;
    Ok(drained)
}

async fn collect_matching_for_message(
    tx: &mut Transaction<'_, Postgres>,
    target_hierarchy: &str,
    target_tag: Option<&str>,
) -> Result<Vec<DrainedRow>, Error> {
    let rows = sqlx::query(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.enqueued_at, \
                p.content \
         FROM message_queue p \
         WHERE p.agent_instance_hierarchy = $1 \
            OR ( \
                p.agent_tag IS NOT NULL \
                AND EXISTS ( \
                    SELECT 1 FROM tags t \
                    WHERE t.name = p.agent_tag \
                      AND t.agent_instance_hierarchy = $1 \
                ) \
            ) \
            OR ( \
                $2::text IS NOT NULL \
                AND p.agent_tag = $2 \
            ) \
         ORDER BY p.id ASC",
    )
    .bind(target_hierarchy)
    .bind(target_tag)
    .fetch_all(&mut **tx)
    .await?;
    rows_to_drained(rows)
}

fn rows_to_drained(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<DrainedRow>, Error> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(DrainedRow {
            message_queue_id: row.try_get(0)?,
            agent_instance_hierarchy: row.try_get(1)?,
            agent_tag: row.try_get(2)?,
            key: row.try_get(3)?,
            enqueued_at: row.try_get(4)?,
            content_json: row.try_get(5)?,
        });
    }
    Ok(out)
}

/// For each matched row, decode the JSON column as `ResponseContent`,
/// reconstruct a `RichContent` from the referenced per-kind rows, then
/// DELETE the message row (which cascades to its content rows via the
/// FK chain).
async fn reconstruct_and_delete(
    tx: &mut Transaction<'_, Postgres>,
    rows: Vec<DrainedRow>,
) -> Result<Vec<DrainedMessage>, Error> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let response_content: ResponseContent = if row.content_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&row.content_json)?
        };
        let content = reconstruct_rich_content(tx, response_content).await?;
        sqlx::query("DELETE FROM message_queue WHERE id = $1")
            .bind(row.message_queue_id)
            .execute(&mut **tx)
            .await?;
        out.push(DrainedMessage {
            agent_instance_hierarchy: row.agent_instance_hierarchy,
            agent_tag: row.agent_tag,
            key: row.key,
            enqueued_at: row.enqueued_at,
            content,
        });
    }
    Ok(out)
}

/// Look up every content row referenced by `rc`, map each to a
/// `RichContentPart`, then bypass `RichContent::from`'s all-text
/// collapse so callers get exactly the shape the queue stored.
async fn reconstruct_rich_content(
    tx: &mut Transaction<'_, Postgres>,
    rc: ResponseContent,
) -> Result<RichContent, Error> {
    let ids: Vec<i64> = match rc {
        ResponseContent::One(id) => vec![id],
        ResponseContent::Many(ids) => ids,
    };
    let mut parts: Vec<RichContentPart> = Vec::with_capacity(ids.len());
    for id in ids {
        let row = read_content_on_conn(&mut **tx, id).await?.ok_or_else(|| {
            Error::InvalidData(format!(
                "queue message referenced missing message_queue_contents id {id}"
            ))
        })?;
        parts.push(content_row_to_part(row));
    }
    // Collapse single-text-part to RichContent::Text (lossless).
    if parts.len() == 1 {
        if let RichContentPart::Text { text } = &parts[0] {
            return Ok(RichContent::Text(text.clone()));
        }
    }
    Ok(RichContent::Parts(parts))
}

// ---------------------------------------------------------------------------
// Delete-by-id — `agents queue delete <id>`.
// ---------------------------------------------------------------------------

/// Atomically delete the `message_queue` row with the given `id` and return
/// its reconstructed shape. `None` when no row matches.
pub async fn delete_by_id(
    pool: &Pool,
    id: i64,
) -> Result<Option<DrainedMessage>, Error> {
    let mut tx = pool.begin().await?;
    let rows = sqlx::query(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.enqueued_at, \
                p.content \
         FROM message_queue p \
         WHERE p.id = $1 \
         ORDER BY p.id ASC",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let drained_rows = rows_to_drained(rows)?;
    let mut items = reconstruct_and_delete(&mut tx, drained_rows).await?;
    tx.commit().await?;
    Ok(items.pop())
}

// ---------------------------------------------------------------------------
// Read-without-delete + clear-by-ids — API-driven split of the drain.
// ---------------------------------------------------------------------------

/// Non-destructive read of every queue row in scope for an agent,
/// fused with the tag-group upgrade. Two-rule predicate: direct
/// hierarchy match OR BOUND-tag match.
///
/// When `agent_tag` is `Some(name)`, this is the **sole** site where
/// the tag-group upgrade fires. The UPDATE runs first inside the
/// transaction so the subsequent SELECT (still in the same tx) sees
/// every freshly-bound sibling tag via rule 2 — committing both
/// effects atomically. When `agent_tag` is `None`, the UPDATE is
/// skipped and only the SELECT runs.
///
/// Upgrade semantics: every `tags` row whose `tag_group` matches
/// `name`'s group flips to BOUND on `target_hierarchy`. If `name`
/// is itself BOUND (or absent), `tag_group` is NULL there, the
/// `WHERE tag_group = (…)` predicate is unknown, and the UPDATE
/// touches nothing — a tag that's already bound is left alone.
pub async fn read_pending_and_upgrade_tag(
    pool: &Pool,
    agent_tag: Option<&str>,
    target_hierarchy: &str,
) -> Result<Vec<(i64, RichContent)>, Error> {
    let mut tx = pool.begin().await?;
    if let Some(tag) = agent_tag {
        let now = now_seconds();
        sqlx::query(
            "UPDATE tags \
             SET agent_instance_hierarchy = $2, \
                 tag_group                = NULL, \
                 updated_at               = $3 \
             WHERE tag_group = ( \
                 SELECT tag_group FROM tags \
                 WHERE name = $1 AND tag_group IS NOT NULL \
             )",
        )
        .bind(tag)
        .bind(target_hierarchy)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    let rows = sqlx::query(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.enqueued_at, \
                p.content \
         FROM message_queue p \
         WHERE p.agent_instance_hierarchy = $1 \
            OR ( \
                p.agent_tag IS NOT NULL \
                AND EXISTS ( \
                    SELECT 1 FROM tags t \
                    WHERE t.name = p.agent_tag \
                      AND t.agent_instance_hierarchy = $1 \
                ) \
            ) \
         ORDER BY p.id ASC",
    )
    .bind(target_hierarchy)
    .fetch_all(&mut *tx)
    .await?;

    let drained = rows_to_drained(rows)?;
    let mut out = Vec::with_capacity(drained.len());
    for row in drained {
        let rc: ResponseContent = if row.content_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&row.content_json)?
        };
        let content = reconstruct_rich_content(&mut tx, rc).await?;
        out.push((row.message_queue_id, content));
    }
    // Commit so the UPGRADE (if any) sticks. The SELECTs in the
    // same tx see the upgrade's effects already.
    tx.commit().await?;
    Ok(out)
}

/// Bulk-delete message rows by id, scoped to the same two-rule
/// predicate `read_pending_and_upgrade_tag` uses. Empty `ids`
/// short-circuits.
pub async fn clear_by_ids(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    ids: Vec<i64>,
) -> Result<(), Error> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    // Loop a single-id DELETE inside one transaction — sqlx doesn't
    // bind `IN (?)` lists from a slice cleanly across drivers, and
    // the per-row scope predicate makes the loop trivial.
    for id in ids {
        sqlx::query(
            "DELETE FROM message_queue \
             WHERE id = $1 \
               AND ( \
                 agent_instance_hierarchy = $2 \
                 OR ( \
                     agent_tag IS NOT NULL \
                     AND EXISTS ( \
                         SELECT 1 FROM tags t \
                         WHERE t.name = agent_tag \
                           AND t.agent_instance_hierarchy = $2 \
                     ) \
                 ) \
               )",
        )
        .bind(id)
        .bind(agent_instance_hierarchy)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Delivery enumeration — `agents queue deliver` fan-out.
// ---------------------------------------------------------------------------

/// Enumerate every distinct `(resolved hierarchy, agent_tag)` pair with
/// pending queue rows in the subtree rooted at `parent` (inclusive).
/// PENDING / ABSENT tag rows are filtered out at the SQL level.
pub async fn list_delivery_targets(
    pool: &Pool,
    parent: &str,
) -> Result<Vec<DeliveryTarget>, Error> {
    let pattern = format!("{parent}/%");
    let rows = sqlx::query(
        "SELECT DISTINCT \
                COALESCE(t.agent_instance_hierarchy, p.agent_instance_hierarchy) AS hier, \
                p.agent_tag \
         FROM message_queue p \
         LEFT JOIN tags t \
             ON p.agent_tag = t.name \
             AND t.agent_instance_hierarchy IS NOT NULL \
         WHERE \
             /* Direct row: target hierarchy in subtree (inclusive). */ \
             ( \
                 p.agent_instance_hierarchy IS NOT NULL \
                 AND ( \
                     p.agent_instance_hierarchy = $1 \
                     OR p.agent_instance_hierarchy LIKE $2 \
                 ) \
             ) \
             OR \
             /* Tag row resolves through a BOUND tag in the subtree. */ \
             ( \
                 p.agent_tag IS NOT NULL \
                 AND t.agent_instance_hierarchy IS NOT NULL \
                 AND ( \
                     t.agent_instance_hierarchy = $1 \
                     OR t.agent_instance_hierarchy LIKE $2 \
                 ) \
             ) \
         ORDER BY hier, p.agent_tag",
    )
    .bind(parent)
    .bind(&pattern)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(DeliveryTarget {
            agent_instance_hierarchy: row.try_get(0)?,
            agent_tag: row.try_get(1)?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Pending-only EXISTS check. Pure read; no tag-group upgrade.
// ---------------------------------------------------------------------------

/// EXISTS-check: are any queue rows in scope for `target_hierarchy`?
///
/// Used by `agents spawn`'s end-of-pass restart logic to
/// decide whether to fire another pass. The two-rule predicate
/// matches `read_pending_and_upgrade_tag`'s SELECT exactly — direct
/// hierarchy hit OR BOUND-tag hit.
///
/// **No upgrade side effect.** Tag-group upgrade now happens
/// exclusively inside `read_pending_and_upgrade_tag`, fired by the
/// conduit on every read-message-queue request. By the time the
/// spawn pass ends, the conduit has already promoted every sibling
/// tag in the group via its own reads, so this pure EXISTS check
/// suffices for the restart decision.
pub async fn check_any_pending(
    pool: &Pool,
    target_hierarchy: &str,
) -> Result<bool, Error> {
    let row = sqlx::query(
        "SELECT EXISTS ( \
             SELECT 1 FROM message_queue p \
             WHERE p.agent_instance_hierarchy = $1 \
                OR ( \
                    p.agent_tag IS NOT NULL \
                    AND EXISTS ( \
                        SELECT 1 FROM tags t \
                        WHERE t.name = p.agent_tag \
                          AND t.agent_instance_hierarchy = $1 \
                    ) \
                ) \
         )",
    )
    .bind(target_hierarchy)
    .fetch_one(&**pool)
    .await?;
    let pending: bool = row.try_get(0)?;
    Ok(pending)
}

// ---------------------------------------------------------------------------
// Delivery subscription — native postgres LISTEN/NOTIFY.
// ---------------------------------------------------------------------------

/// Wait until the `message_queue` row identified by `id` is
/// deleted. Resolves `Ok(())` the moment the row is gone — either
/// because the conduit's `clear_by_ids` just removed it, or
/// because it was already gone before we started listening.
///
/// Uses `sqlx::postgres::PgListener` on the
/// `message_queue_delete` channel that the AFTER-DELETE trigger
/// in `db::init` populates. The function attaches the listener
/// FIRST, then re-checks whether the row still exists — that's
/// what closes the window where a fast delete races our
/// `LISTEN`.
pub async fn subscribe_delivered(pool: &Pool, id: i64) -> Result<(), Error> {
    use sqlx::postgres::PgListener;

    let mut listener = PgListener::connect_with(&**pool).await?;
    listener.listen("message_queue_delete").await?;

    // Belt-and-suspenders: if the row is already gone (the
    // conduit raced our listen), the LISTEN saw nothing and would
    // hang forever. SELECT once after attaching — if the row is
    // gone, we already delivered. After this point the LISTEN is
    // attached so any future DELETE will wake us.
    let still_present: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM message_queue WHERE id = $1)",
    )
    .bind(id)
    .fetch_one(&**pool)
    .await?;
    if !still_present {
        return Ok(());
    }

    let target = id.to_string();
    loop {
        let notification = listener.recv().await?;
        if notification.payload() == target {
            return Ok(());
        }
        // Different row — keep listening.
    }
}
