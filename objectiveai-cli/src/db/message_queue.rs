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
use objectiveai_sdk::cli::command::agents::queue::read::pending::{
    QueuePart, QueuePartType, ResponseItem,
};
use objectiveai_sdk::client_objectiveai_mcp::server_response::ReadMessageQueueResult;
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
    sender_agent_instance_hierarchy: &str,
    key: Option<String>,
    content: RichContent,
) -> Result<i64, Error> {
    let mut tx = pool.begin().await?;
    let message_queue_id = enqueue_with_content_in_tx(
        &mut tx,
        agent_instance_hierarchy.as_deref(),
        agent_tag.as_deref(),
        sender_agent_instance_hierarchy,
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
    sender_agent_instance_hierarchy: &str,
    key: Option<&str>,
    enqueued_at: i64,
    content: RichContent,
) -> Result<i64, Error> {
    if let Some(key_value) = key {
        // Upsert via soft-flip: any prior active row for this
        // (target, key) pair flips to inactive so the partial
        // unique index (now `WHERE … AND active = TRUE`) lets the
        // new INSERT through. The flipped row + its
        // `message_queue_contents` children survive untouched —
        // they're invisible to readers via `active = FALSE`.
        sqlx::query(
            "UPDATE message_queue SET active = FALSE \
             WHERE active = TRUE \
               AND key = $3 \
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
    let message_queue_id: i64 = sqlx::query_scalar(
        "INSERT INTO message_queue \
            (agent_instance_hierarchy, agent_tag, \
             sender_agent_instance_hierarchy, enqueued_at, key) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id",
    )
    .bind(agent_instance_hierarchy)
    .bind(agent_tag)
    .bind(sender_agent_instance_hierarchy)
    .bind(enqueued_at)
    .bind(key)
    .fetch_one(&mut **tx)
    .await?;
    walk_rich(tx, message_queue_id, content).await?;
    Ok(message_queue_id)
}

/// Insert each content slot into the matching per-kind table.
/// All rows link back to `message_queue_id` via the FK on
/// `message_queue_contents`; reads JOIN, never look at the parent
/// row for content. No JSON shadow of the slot ids is written
/// anywhere.
async fn walk_rich(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
    content: RichContent,
) -> Result<(), Error> {
    match content {
        RichContent::Text(text) => {
            insert_content_text(tx, message_queue_id, &text).await?;
        }
        RichContent::Parts(parts) => {
            for part in parts {
                insert_content_part(tx, message_queue_id, part).await?;
            }
        }
    }
    Ok(())
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
/// One target the CLI handler has already resolved. `Hierarchy`
/// is a direct AIH; `Tag` is a tag-name post-`tags::lookup`. The
/// SQL filters by whichever column matches.
#[derive(Debug, Clone)]
pub enum ResolvedTarget {
    Hierarchy(String),
    Tag(String),
}

/// Stream every `active = TRUE` `message_queue` row for any of
/// `targets`, JOINed against `message_queue_contents` for parts.
/// Grouped per-parent so `parts` reflects the relational
/// content shape, not a JSON shadow. Pagination is by
/// `message_queue_contents.id` (the `--after-id` / `--limit`
/// scope is on parts, not rows — but the row boundary stays
/// stable per parent).
pub async fn list_pending_for_targets(
    pool: &Pool,
    targets: &[ResolvedTarget],
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ResponseItem>, Error> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let mut hier_targets: Vec<String> = Vec::new();
    let mut tag_targets: Vec<String> = Vec::new();
    for t in targets {
        match t {
            ResolvedTarget::Hierarchy(h) => hier_targets.push(h.clone()),
            ResolvedTarget::Tag(t) => tag_targets.push(t.clone()),
        }
    }
    let rows = sqlx::query(
        "SELECT p.id                              AS message_queue_id, \
                p.agent_instance_hierarchy        AS agent_instance_hierarchy, \
                p.agent_tag                       AS agent_tag, \
                p.sender_agent_instance_hierarchy AS sender_agent_instance_hierarchy, \
                p.enqueued_at                     AS timestamp_queued, \
                p.key                             AS key, \
                c.id                              AS content_id, \
                c.kind                            AS content_kind \
         FROM message_queue p \
         JOIN message_queue_contents c ON c.message_queue_id = p.id \
         WHERE p.active = TRUE \
           AND ( \
                ( $1::text[] IS NOT NULL \
                  AND p.agent_instance_hierarchy = ANY($1) ) \
             OR ( $2::text[] IS NOT NULL \
                  AND p.agent_tag = ANY($2) ) \
           ) \
           AND c.id > COALESCE($3, 0) \
         ORDER BY p.id ASC, c.id ASC \
         LIMIT COALESCE($4, 1000)",
    )
    .bind(if hier_targets.is_empty() { None } else { Some(&hier_targets) })
    .bind(if tag_targets.is_empty() { None } else { Some(&tag_targets) })
    .bind(after_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await?;

    // Walk the joined rows and group consecutive rows that share
    // `message_queue_id` into one ResponseItem. Each row is one
    // content slot; the parent's metadata (sender, timestamp,
    // key) is identical across all of a parent's rows.
    let mut out: Vec<ResponseItem> = Vec::new();
    let mut cur_parent: Option<i64> = None;
    let mut cur_aih: Option<String> = None;
    let mut cur_tag: Option<String> = None;
    let mut cur_sender: String = String::new();
    let mut cur_timestamp: i64 = 0;
    let mut cur_key: Option<String> = None;
    let mut cur_parts: Vec<QueuePart> = Vec::new();

    let flush = |parent: &mut Option<i64>,
                 aih: &mut Option<String>,
                 tag: &mut Option<String>,
                 sender: &mut String,
                 timestamp: &mut i64,
                 key: &mut Option<String>,
                 parts: &mut Vec<QueuePart>,
                 out: &mut Vec<ResponseItem>| {
        if parts.is_empty() {
            return;
        }
        let parts = std::mem::take(parts);
        let sender = std::mem::take(sender);
        let key = key.take();
        let timestamp_queued = *timestamp;
        let delete_id = parent.take().unwrap_or_default();
        if let Some(h) = aih.take() {
            out.push(ResponseItem::AgentInstanceHierarchy {
                delete_id,
                agent_instance_hierarchy: h,
                sender_agent_instance_hierarchy: sender,
                timestamp_queued,
                key,
                parts,
            });
        } else if let Some(t) = tag.take() {
            out.push(ResponseItem::Tag {
                delete_id,
                agent_tag: t,
                sender_agent_instance_hierarchy: sender,
                timestamp_queued,
                key,
                parts,
            });
        }
        *timestamp = 0;
    };

    for row in rows {
        let parent_id: i64 = row.try_get("message_queue_id")?;
        if cur_parent != Some(parent_id) {
            flush(
                &mut cur_parent, &mut cur_aih, &mut cur_tag,
                &mut cur_sender, &mut cur_timestamp, &mut cur_key,
                &mut cur_parts, &mut out,
            );
            cur_parent = Some(parent_id);
            cur_aih = row.try_get("agent_instance_hierarchy")?;
            cur_tag = row.try_get("agent_tag")?;
            cur_sender = row.try_get("sender_agent_instance_hierarchy")?;
            cur_timestamp = row.try_get("timestamp_queued")?;
            cur_key = row.try_get("key")?;
        }
        let content_id: i64 = row.try_get("content_id")?;
        let kind: String = row.try_get("content_kind")?;
        let r#type = match kind.as_str() {
            "text"  => QueuePartType::Text,
            "image" => QueuePartType::Image,
            "audio" => QueuePartType::Audio,
            "video" => QueuePartType::Video,
            "file"  => QueuePartType::File,
            other => return Err(Error::InvalidData(format!(
                "unknown message_queue_contents.kind: {other}"
            ))),
        };
        cur_parts.push(QueuePart { id: content_id, r#type });
    }
    flush(
        &mut cur_parent, &mut cur_aih, &mut cur_tag,
        &mut cur_sender, &mut cur_timestamp, &mut cur_key,
        &mut cur_parts, &mut out,
    );
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
                p.enqueued_at \
         FROM message_queue p \
         WHERE p.active = TRUE \
           AND ( \
                p.agent_instance_hierarchy = $1 \
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
            ) ) \
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
        });
    }
    Ok(out)
}

/// For each matched row, look up its content rows by
/// `message_queue_id`, reconstruct a `RichContent`, then
/// soft-flip the parent row (`active = FALSE`). The parent row
/// and its `message_queue_contents` children survive untouched
/// — invisible to readers via `active = FALSE`.
async fn reconstruct_and_delete(
    tx: &mut Transaction<'_, Postgres>,
    rows: Vec<DrainedRow>,
) -> Result<Vec<DrainedMessage>, Error> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let content = reconstruct_rich_content(tx, row.message_queue_id).await?;
        sqlx::query(
            "UPDATE message_queue SET active = FALSE \
             WHERE id = $1 AND active = TRUE",
        )
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

/// Pull every `message_queue_contents` row whose
/// `message_queue_id` matches and assemble a `RichContent` in
/// id-ASC order. Single-text-part collapses to
/// `RichContent::Text` so callers see the exact shape the queue
/// stored.
async fn reconstruct_rich_content(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
) -> Result<RichContent, Error> {
    let (_, parts) = fetch_content_parts_for_queue_id(tx, message_queue_id).await?;
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

/// Atomically soft-delete the `message_queue` row with the given
/// `id` (flips `active = FALSE`) and return its reconstructed
/// shape. `None` when no active row matches. The row + its
/// `message_queue_contents` children survive in the table for
/// audit purposes — they're invisible to readers via
/// `active = FALSE`.
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
                p.enqueued_at \
         FROM message_queue p \
         WHERE p.id = $1 AND p.active = TRUE \
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
) -> Result<ReadMessageQueueResult, Error> {
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
                p.enqueued_at \
         FROM message_queue p \
         WHERE p.active = TRUE \
           AND ( \
                p.agent_instance_hierarchy = $1 \
             OR ( \
                p.agent_tag IS NOT NULL \
                AND EXISTS ( \
                    SELECT 1 FROM tags t \
                    WHERE t.name = p.agent_tag \
                      AND t.agent_instance_hierarchy = $1 \
                ) \
            ) ) \
         ORDER BY p.id ASC",
    )
    .bind(target_hierarchy)
    .fetch_all(&mut *tx)
    .await?;

    // Join every entry into one RichContent with `"\n\n"`
    // separators between entries; collect the flat list of
    // `message_queue_contents.id`s in consumption order. The
    // count may differ from `rich_content`'s part count because
    // of separator insertion + the single-text-part collapse path
    // below. Kinds are resolved CLI-side at log-write time, so
    // they don't ride on the wire.
    let drained = rows_to_drained(rows)?;
    let mut all_parts: Vec<RichContentPart> = Vec::new();
    let mut all_ids: Vec<i64> = Vec::new();
    for (i, row) in drained.into_iter().enumerate() {
        if i > 0 {
            all_parts.push(RichContentPart::Text {
                text: "\n\n".to_string(),
            });
        }
        let (content_ids, parts) =
            fetch_content_parts_for_queue_id(&mut tx, row.message_queue_id).await?;
        all_ids.extend(content_ids);
        all_parts.extend(parts);
    }
    // Collapse single-text-part to RichContent::Text (lossless).
    let rich_content = if all_parts.len() == 1
        && matches!(all_parts.first(), Some(RichContentPart::Text { .. }))
    {
        let Some(RichContentPart::Text { text }) = all_parts.into_iter().next()
        else {
            unreachable!("matched single Text part above")
        };
        RichContent::Text(text)
    } else {
        RichContent::Parts(all_parts)
    };
    // Commit so the UPGRADE (if any) sticks. The SELECTs in the
    // same tx see the upgrade's effects already.
    tx.commit().await?;
    Ok(ReadMessageQueueResult {
        rich_content,
        ids: all_ids,
    })
}

/// Like `reconstruct_rich_content` but returns the consumed
/// `message_queue_contents.id`s alongside the rich-content parts
/// so the caller can preserve content-id provenance through the
/// join + separator path. Kinds aren't returned — the LogWriter
/// resolves them at write time via SQL CASE.
/// Look up every `message_queue_contents` row whose
/// `message_queue_id` matches and return (ids, RichContentParts)
/// in id ASC order. Replaces the old JSON-shadow path — content
/// is fetched relationally, never via a denormalized parent
/// column.
async fn fetch_content_parts_for_queue_id(
    tx: &mut Transaction<'_, Postgres>,
    message_queue_id: i64,
) -> Result<(Vec<i64>, Vec<RichContentPart>), Error> {
    let id_rows = sqlx::query(
        "SELECT id FROM message_queue_contents \
         WHERE message_queue_id = $1 \
         ORDER BY id ASC",
    )
    .bind(message_queue_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut out_ids = Vec::with_capacity(id_rows.len());
    let mut out_parts = Vec::with_capacity(id_rows.len());
    for r in id_rows {
        let id: i64 = r.try_get("id")?;
        let row = read_content_on_conn(&mut **tx, id).await?.ok_or_else(|| {
            Error::InvalidData(format!(
                "message_queue_contents id {id} disappeared mid-tx"
            ))
        })?;
        out_ids.push(id);
        out_parts.push(content_row_to_part(row));
    }
    Ok((out_ids, out_parts))
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
         WHERE p.active = TRUE AND ( \
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
             ) ) \
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
             WHERE p.active = TRUE \
               AND ( \
                    p.agent_instance_hierarchy = $1 \
                 OR ( \
                    p.agent_tag IS NOT NULL \
                    AND EXISTS ( \
                        SELECT 1 FROM tags t \
                        WHERE t.name = p.agent_tag \
                          AND t.agent_instance_hierarchy = $1 \
                    ) \
                ) ) \
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

/// Wait until the `message_queue` row identified by `id` has been
/// consumed — i.e. its `active` column has flipped from TRUE to
/// FALSE. Resolves `Ok(())` the moment the flip is observed,
/// regardless of which path performed it.
///
/// Uses `sqlx::postgres::PgListener` on the
/// `message_queue_inactive` channel that the AFTER-UPDATE trigger
/// in `db::init` populates. The function attaches the listener
/// FIRST, then re-checks whether the row is still active — that's
/// what closes the window where a fast flip races our `LISTEN`.
pub async fn subscribe_delivered(pool: &Pool, id: i64) -> Result<(), Error> {
    use sqlx::postgres::PgListener;

    let mut listener = PgListener::connect_with(&**pool).await?;
    listener.listen("message_queue_inactive").await?;

    // Belt-and-suspenders: if the row already flipped to inactive
    // (the conduit / LogWriter raced our listen), the LISTEN saw
    // nothing and would hang forever. SELECT once after attaching
    // — if the row is gone or already inactive, we already
    // delivered. After this point the LISTEN is attached so any
    // future flip will wake us.
    let still_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM message_queue \
         WHERE id = $1 AND active = TRUE)",
    )
    .bind(id)
    .fetch_one(&**pool)
    .await?;
    if !still_active {
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
