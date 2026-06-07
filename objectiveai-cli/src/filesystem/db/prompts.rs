//! Deferred-prompt storage for `agents message-queue {add, list, read id}`.
//!
//! Co-located in `tags.sqlite` with the `tags` table so the
//! queue-list leaf can JOIN prompts ⨝ tags in a single SELECT (to
//! surface each tag-keyed row's BOUND / PENDING state). The
//! connection slot is owned by [`super::tags`]; this module
//! piggybacks on it.
//!
//! ## Schema
//!
//! `prompts` — one row per queued prompt (which is a single
//! user-message-equivalent `RichContent`), targeting either an
//! `agent_instance_hierarchy` OR an `agent_tag` (never both —
//! `CHECK` enforces it). The `prompt` column holds the JSON
//! serialization of one [`ResponseContent`] — either `One(i64)`
//! for single-part content or `Many(Vec<i64>)` for multi-part —
//! referencing rows in the per-kind content tables below.
//!
//! `prompt_contents` — master content registry, FK-anchored at
//! `prompt_id` so a single `DELETE FROM prompts WHERE id = ?`
//! cascades the entire prompt out. Per-kind tables (`prompt_texts`,
//! `prompt_images`, `prompt_audios`, `prompt_videos`,
//! `prompt_files`) share the master's row id 1:1 and cascade
//! again on the per-kind FK.

use objectiveai_sdk::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContent, RichContentPart, VideoUrl,
};
use objectiveai_sdk::cli::command::agents::message_queue::read::pending::{
    LookupState, ResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::read::all::ResponseContent;
use rusqlite::{Connection, OptionalExtension as _, params};

use super::super::{Client, Error};

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert a `prompts` row directly. Used by tests (and as the low-
/// level escape hatch). Production callers must go through
/// [`enqueue_with_content_async`] so the `prompt` column carries the
/// id-referenced `Vec<ResponseQueueMessage>` shape rather than a raw
/// `Vec<Message>` JSON blob.
pub fn insert(
    client: &Client,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    prompt: &str,
) -> Result<i64, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "INSERT INTO prompts (agent_instance_hierarchy, agent_tag, prompt, enqueued_at) \
         VALUES (?1, ?2, ?3, ?4) \
         RETURNING id",
    )?;
    let id = stmt.query_row(
        params![agent_instance_hierarchy, agent_tag, prompt, now_seconds()],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(id)
}

/// Async wrapper around [`insert`].
pub async fn insert_async(
    client: Client,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    prompt: String,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        insert(
            &client,
            agent_instance_hierarchy.as_deref(),
            agent_tag.as_deref(),
            &prompt,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// One content row — typed payload of a single `prompt_contents.id`.
/// Returned by [`read_content`] / [`read_content_async`].
///
/// Variants map one-to-one to `prompt_contents.kind`. The
/// `CHECK (kind IN ('text','image','audio','video','file'))`
/// constraint on the master table guarantees the five variants here
/// are exhaustive — no assistant-side kinds (reasoning, refusal,
/// tool_call) exist for queue content because the queue stores one
/// user-message-equivalent `RichContent` per row, not arbitrary
/// conversation history.
#[derive(Debug, Clone)]
pub enum ContentRow {
    Text(String),
    Image(ImageUrl),
    Audio(InputAudio),
    Video(VideoUrl),
    File(File),
}

/// Map one [`ContentRow`] to its matching SDK [`RichContentPart`]
/// variant. Shared by `agents message-queue read id`'s handler (single-id
/// fetch) and the drain helpers below (reconstruct a whole prompt).
/// The walker stored both `InputVideo` and `VideoUrl` parts as
/// `prompt_videos` (just a URL), so reading back always yields
/// `VideoUrl` — the lossless choice for a bare URL.
pub fn content_row_to_part(row: ContentRow) -> RichContentPart {
    match row {
        ContentRow::Text(text) => RichContentPart::Text { text },
        ContentRow::Image(image_url) => RichContentPart::ImageUrl { image_url },
        ContentRow::Audio(input_audio) => RichContentPart::InputAudio { input_audio },
        ContentRow::Video(video_url) => RichContentPart::VideoUrl { video_url },
        ContentRow::File(file) => RichContentPart::File { file },
    }
}

/// Look up a single content row by `prompt_contents.id`. Returns
/// `None` when the id doesn't exist (`prompt_contents` miss) — a
/// per-kind miss is a DB corruption and is reported as an `Error`.
///
/// Locking wrapper around [`read_content_with_conn`]; callers that
/// already hold the `tags` connection (e.g. inside a drain
/// transaction) should call the `_with_conn` variant directly.
pub fn read_content(client: &Client, id: i64) -> Result<Option<ContentRow>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    read_content_with_conn(&conn, id)
}

/// `read_content` that operates on an externally-acquired
/// connection (or [`rusqlite::Transaction`], which derefs to
/// `&Connection`). Used by the drain helpers below so reconstruction
/// composes inside the surrounding transaction.
pub fn read_content_with_conn(
    conn: &Connection,
    id: i64,
) -> Result<Option<ContentRow>, Error> {
    let kind: Option<String> = conn
        .query_row(
            "SELECT kind FROM prompt_contents WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(kind) = kind else {
        return Ok(None);
    };
    let row = match kind.as_str() {
        "text" => {
            let text: String = conn.query_row(
                "SELECT text FROM prompt_texts WHERE id = ?1",
                [id],
                |r| r.get(0),
            )?;
            ContentRow::Text(text)
        }
        "image" => {
            let (url, detail): (String, Option<String>) = conn.query_row(
                "SELECT url, detail FROM prompt_images WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let detail = match detail {
                Some(s) => serde_json::from_value(serde_json::Value::String(s))
                    .map_err(Error::Json)?,
                None => None,
            };
            ContentRow::Image(ImageUrl { url, detail })
        }
        "audio" => {
            let (data, format): (String, String) = conn.query_row(
                "SELECT data, format FROM prompt_audios WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            ContentRow::Audio(InputAudio { data, format })
        }
        "video" => {
            let url: String = conn.query_row(
                "SELECT url FROM prompt_videos WHERE id = ?1",
                [id],
                |r| r.get(0),
            )?;
            ContentRow::Video(VideoUrl { url })
        }
        "file" => {
            let (file_data, file_id, filename, file_url): (
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ) = conn.query_row(
                "SELECT file_data, file_id, filename, file_url \
                 FROM prompt_files WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
            ContentRow::File(File {
                file_data,
                file_id,
                filename,
                file_url,
            })
        }
        // CHECK constraint guards against unknown kind values at
        // insert time; a miss here is DB corruption.
        other => {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown prompt_contents.kind: {other}"),
            )));
        }
    };
    Ok(Some(row))
}

/// Async wrapper around [`read_content`].
pub async fn read_content_async(
    client: Client,
    id: i64,
) -> Result<Option<ContentRow>, Error> {
    tokio::task::spawn_blocking(move || read_content(&client, id))
        .await
        .map_err(spawn_blocking_join_err)?
}

// ---------------------------------------------------------------------------
// Transactional walker — Vec<Message> → (prompt_id, Vec<ResponseQueueMessage>)
// ---------------------------------------------------------------------------

/// Atomic enqueue: inserts the `prompts` row, walks `content` and
/// extracts every part into a per-kind table referenced by id, then
/// UPDATEs the `prompts.prompt` column with the assembled
/// [`ResponseContent`] JSON (`One(i64)` for single-part,
/// `Many(Vec<i64>)` for multi-part). Returns the new `prompts.id`.
///
/// Everything runs inside one rusqlite transaction on the shared
/// `tags.sqlite` connection — any failure rolls the prompt and all
/// its content rows back, leaving no orphans.
pub async fn enqueue_with_content_async(
    client: Client,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    key: Option<String>,
    content: RichContent,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        enqueue_with_content(
            &client,
            agent_instance_hierarchy.as_deref(),
            agent_tag.as_deref(),
            key.as_deref(),
            content,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn enqueue_with_content(
    client: &Client,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    key: Option<&str>,
    content: RichContent,
) -> Result<i64, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    let prompt_id = enqueue_with_content_in_tx(
        &tx,
        agent_instance_hierarchy,
        agent_tag,
        key,
        now_seconds(),
        content,
    )?;
    tx.commit()?;
    Ok(prompt_id)
}

/// Insert one prompt row + its content rows inside an existing
/// transaction. Lets `re_enqueue_async` batch multiple inserts
/// under a single transaction by reusing this body once per item.
/// `enqueued_at` is parameterised so re-enqueue can preserve the
/// originally-stored timestamp (and FIFO order across drain → fail
/// → re-enqueue cycles).
fn enqueue_with_content_in_tx(
    tx: &rusqlite::Transaction<'_>,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    key: Option<&str>,
    enqueued_at: i64,
    content: RichContent,
) -> Result<i64, Error> {
    if let Some(key_value) = key {
        // Upsert: drop any prior row for this (target, key) pair so
        // the partial unique index never trips. Cascade on
        // `prompt_contents.prompt_id` sweeps the prior row's
        // content rows in the same transaction.
        tx.execute(
            "DELETE FROM prompts \
             WHERE key = ?3 \
               AND ( \
                   (agent_instance_hierarchy IS NOT NULL \
                    AND ?1 IS NOT NULL \
                    AND agent_instance_hierarchy = ?1) \
                   OR \
                   (agent_tag IS NOT NULL \
                    AND ?2 IS NOT NULL \
                    AND agent_tag = ?2) \
               )",
            params![agent_instance_hierarchy, agent_tag, key_value],
        )?;
    }
    // Empty `prompt` placeholder — overwritten by the final UPDATE
    // once we know the id-referenced shape. `prompts.prompt` is
    // NOT NULL so we need *some* value here.
    let prompt_id: i64 = tx.query_row(
        "INSERT INTO prompts (agent_instance_hierarchy, agent_tag, prompt, enqueued_at, key) \
         VALUES (?1, ?2, '', ?3, ?4) \
         RETURNING id",
        params![agent_instance_hierarchy, agent_tag, enqueued_at, key],
        |r| r.get(0),
    )?;
    let response_content = walk_rich(tx, prompt_id, content)?;
    let json = serde_json::to_string(&response_content).map_err(Error::Json)?;
    tx.execute(
        "UPDATE prompts SET prompt = ?1 WHERE id = ?2",
        params![json, prompt_id],
    )?;
    Ok(prompt_id)
}

fn walk_rich(
    tx: &rusqlite::Transaction<'_>,
    prompt_id: i64,
    content: RichContent,
) -> Result<ResponseContent, Error> {
    match content {
        RichContent::Text(text) => {
            let id = insert_content_text(tx, prompt_id, &text)?;
            Ok(ResponseContent::One(id))
        }
        RichContent::Parts(parts) => {
            let mut ids = Vec::with_capacity(parts.len());
            for part in parts {
                ids.push(insert_content_part(tx, prompt_id, part)?);
            }
            if ids.len() == 1 {
                Ok(ResponseContent::One(ids.remove(0)))
            } else {
                Ok(ResponseContent::Many(ids))
            }
        }
    }
}

fn insert_content_part(
    tx: &rusqlite::Transaction<'_>,
    prompt_id: i64,
    part: RichContentPart,
) -> Result<i64, Error> {
    match part {
        RichContentPart::Text { text } => insert_content_text(tx, prompt_id, &text),
        RichContentPart::ImageUrl { image_url } => {
            insert_content_image(tx, prompt_id, &image_url)
        }
        RichContentPart::InputAudio { input_audio } => {
            insert_content_audio(tx, prompt_id, &input_audio)
        }
        RichContentPart::InputVideo { video_url }
        | RichContentPart::VideoUrl { video_url } => {
            insert_content_video(tx, prompt_id, &video_url)
        }
        RichContentPart::File { file } => insert_content_file(tx, prompt_id, &file),
    }
}

// ---------------------------------------------------------------------------
// Per-kind insert helpers — each mints a `prompt_contents.id` for the
// caller-supplied `prompt_id`, then inserts the per-kind row sharing
// that id. All helpers take `&Connection` so they compose inside the
// `enqueue_with_content` transaction.
// ---------------------------------------------------------------------------

fn mint_content_id(
    conn: &Connection,
    prompt_id: i64,
    kind: &str,
) -> Result<i64, Error> {
    Ok(conn.query_row(
        "INSERT INTO prompt_contents (prompt_id, kind) VALUES (?1, ?2) RETURNING id",
        params![prompt_id, kind],
        |r| r.get::<_, i64>(0),
    )?)
}

fn insert_content_text(
    conn: &Connection,
    prompt_id: i64,
    text: &str,
) -> Result<i64, Error> {
    let id = mint_content_id(conn, prompt_id, "text")?;
    conn.execute(
        "INSERT INTO prompt_texts (id, text) VALUES (?1, ?2)",
        params![id, text],
    )?;
    Ok(id)
}

fn insert_content_image(
    conn: &Connection,
    prompt_id: i64,
    image: &ImageUrl,
) -> Result<i64, Error> {
    let id = mint_content_id(conn, prompt_id, "image")?;
    let detail = image
        .detail
        .as_ref()
        .map(|d| serde_json::to_value(d).map(|v| v.as_str().map(str::to_string)))
        .transpose()
        .map_err(Error::Json)?
        .flatten();
    conn.execute(
        "INSERT INTO prompt_images (id, url, detail) VALUES (?1, ?2, ?3)",
        params![id, image.url, detail],
    )?;
    Ok(id)
}

fn insert_content_audio(
    conn: &Connection,
    prompt_id: i64,
    audio: &InputAudio,
) -> Result<i64, Error> {
    let id = mint_content_id(conn, prompt_id, "audio")?;
    conn.execute(
        "INSERT INTO prompt_audios (id, data, format) VALUES (?1, ?2, ?3)",
        params![id, audio.data, audio.format],
    )?;
    Ok(id)
}

fn insert_content_video(
    conn: &Connection,
    prompt_id: i64,
    video: &VideoUrl,
) -> Result<i64, Error> {
    let id = mint_content_id(conn, prompt_id, "video")?;
    conn.execute(
        "INSERT INTO prompt_videos (id, url) VALUES (?1, ?2)",
        params![id, video.url],
    )?;
    Ok(id)
}

fn insert_content_file(
    conn: &Connection,
    prompt_id: i64,
    file: &File,
) -> Result<i64, Error> {
    let id = mint_content_id(conn, prompt_id, "file")?;
    conn.execute(
        "INSERT INTO prompt_files (id, file_data, file_id, filename, file_url) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, file.file_data, file.file_id, file.filename, file.file_url],
    )?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// List all queued prompts visible under `parent`. Every row —
/// Direct or Tag — must resolve to a parent matching the filter:
///
/// * **Direct rows:** `agent_instance_hierarchy` is a direct child
///   of `parent` (the same `LIKE 'parent/%' AND no_further_slash`
///   pattern `list_direct_active_children` uses).
/// * **BOUND tag rows (LEFT JOIN finds the tag):** the tag's bound
///   `agent_instance_hierarchy` is a direct child of `parent` — same
///   predicate, applied against the joined `t.agent_instance_hierarchy`.
/// * **PENDING tag rows:** the tag's stored
///   `parent_agent_instance_hierarchy` equals `parent` exactly.
/// * **ABSENT tag rows (LEFT JOIN returns no match):** excluded —
///   no parent info available.
///
/// Each returned item embeds the prompt body as a
/// [`ResponseContent`] (`One(i64)` or `Many(Vec<i64>)`, decoded
/// directly from the `prompts.prompt` JSON column) so callers can
/// fan out to `agents message-queue read id` per piece without re-fetching
/// the prompt.
///
/// Returned items are ordered by `prompts.id` (FIFO of enqueue).
pub fn list(client: &Client, parent: &str) -> Result<Vec<ResponseItem>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.prompt, \
                t.agent_instance_hierarchy        AS tag_bound_hierarchy, \
                t.parent_agent_instance_hierarchy AS tag_pending_parent, \
                t.agent_full_id                   AS tag_pending_full_id \
         FROM prompts p \
         LEFT JOIN tags t ON p.agent_tag = t.name \
         WHERE \
                /* Direct row: agent_instance_hierarchy is a direct child of ?1 */ \
                ( \
                    p.agent_instance_hierarchy IS NOT NULL \
                    AND p.agent_instance_hierarchy LIKE (?1 || '/%') \
                    AND instr(substr(p.agent_instance_hierarchy, length(?1) + 2), '/') = 0 \
                ) \
             /* BOUND tag: tag's bound hierarchy is a direct child of ?1 */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND t.agent_instance_hierarchy IS NOT NULL \
                    AND t.agent_instance_hierarchy LIKE (?1 || '/%') \
                    AND instr(substr(t.agent_instance_hierarchy, length(?1) + 2), '/') = 0 \
                ) \
             /* PENDING tag: tag's stored parent equals ?1 exactly */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND t.parent_agent_instance_hierarchy = ?1 \
                ) \
         ORDER BY p.id",
    )?;
    let rows = stmt
        .query_map([parent], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (
        id,
        agent_instance_hierarchy,
        agent_tag,
        key,
        prompt_json,
        tag_bound_hierarchy,
        tag_pending_parent,
        tag_pending_full_id,
    ) in rows
    {
        // The placeholder empty string can briefly exist mid-
        // transaction, but a committed row always carries the final
        // ResponseContent JSON. Treat empty defensively as One(0) →
        // really a corrupted row; we surface it as Many([]) which
        // serialises cleanly and is harmless downstream.
        let content: ResponseContent = if prompt_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&prompt_json).map_err(Error::Json)?
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
            let Some(state) = (match (tag_bound_hierarchy, tag_pending_parent, tag_pending_full_id)
            {
                (Some(agent_instance_hierarchy), None, None) => Some(LookupState::Bound {
                    agent_instance_hierarchy,
                }),
                (None, Some(parent_agent_instance_hierarchy), Some(agent_full_id)) => {
                    Some(LookupState::Pending {
                        parent_agent_instance_hierarchy,
                        agent_full_id,
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

/// Async wrapper around [`list`].
pub async fn list_async(client: Client, parent: String) -> Result<Vec<ResponseItem>, Error> {
    tokio::task::spawn_blocking(move || list(&client, &parent))
        .await
        .map_err(spawn_blocking_join_err)?
}

// ---------------------------------------------------------------------------
// Drain — atomically pull matching prompts out of the queue and
// reconstruct each one as a `RichContent`. Used by `agents message`
// and `agents spawn` to prepend queued content to the user's own
// payload before the API call fires. If the surrounding call fails
// before its first OK stream item, the caller hands the
// `Vec<DrainedPrompt>` back to [`re_enqueue_async`] to restore the
// queue state.
// ---------------------------------------------------------------------------

/// One drained prompt — carries enough metadata to re-INSERT the
/// original row (same target columns, same `enqueued_at`) plus the
/// reconstructed body for the join-and-prepend path. The caller
/// joins `content`s into the outgoing message and keeps the
/// `Vec<DrainedPrompt>` around for the rollback hook.
#[derive(Debug, Clone)]
pub struct DrainedPrompt {
    /// Original `prompts.agent_instance_hierarchy`; `Some` when the
    /// drained row was Direct-targeted.
    pub agent_instance_hierarchy: Option<String>,
    /// Original `prompts.agent_tag`; `Some` when the drained row
    /// was Tag-targeted.
    pub agent_tag: Option<String>,
    /// Original `prompts.key`. Preserved through re-enqueue so a
    /// failed delivery doesn't lose the idempotency token (#213).
    /// `None` for unkeyed rows.
    pub key: Option<String>,
    /// Original `prompts.enqueued_at`. Preserved through re-enqueue
    /// so FIFO ordering survives a drain → fail → re-enqueue cycle.
    pub enqueued_at: i64,
    /// Reconstructed body — used by the join-and-prepend path today,
    /// and (in identical form) by re-enqueue's walker on failure.
    pub content: RichContent,
}

/// Drain rows targeting `target_hierarchy`, `target_tag` (if some),
/// or any BOUND tag whose `agent_instance_hierarchy` equals
/// `target_hierarchy`. Returns drained prompts in enqueue
/// (oldest-first) order; deletes the matched rows in the same
/// transaction.
pub async fn drain_for_message_async(
    client: Client,
    target_hierarchy: String,
    target_tag: Option<String>,
) -> Result<Vec<DrainedPrompt>, Error> {
    tokio::task::spawn_blocking(move || {
        drain_for_message(&client, &target_hierarchy, target_tag.as_deref())
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn drain_for_message(
    client: &Client,
    target_hierarchy: &str,
    target_tag: Option<&str>,
) -> Result<Vec<DrainedPrompt>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    // Three-rule predicate (see plan):
    //  1. p.agent_instance_hierarchy = ?1
    //  2. p.agent_tag → tag BOUND to ?1
    //  3. ?2 is some AND p.agent_tag = ?2
    // ?2 may be NULL — rule 3 silently fails the AND in that case.
    let rows = collect_matching_prompts(
        &tx,
        "p.agent_instance_hierarchy = ?1 \
         OR ( \
             p.agent_tag IS NOT NULL \
             AND EXISTS ( \
                 SELECT 1 FROM tags t \
                 WHERE t.name = p.agent_tag \
                   AND t.agent_instance_hierarchy = ?1 \
             ) \
         ) \
         OR ( \
             ?2 IS NOT NULL \
             AND p.agent_tag = ?2 \
         )",
        params![target_hierarchy, target_tag],
    )?;
    let drained = reconstruct_and_delete(&tx, rows)?;
    tx.commit()?;
    Ok(drained)
}

/// Drain rows targeting `target_tag` (if some), or any PENDING tag
/// whose `(parent_agent_instance_hierarchy, agent_full_id)` pair
/// matches `(parent_hierarchy, agent_full_id)` — i.e. every tag
/// that the spawn about to fire will auto-promote on first chunk.
pub async fn drain_for_spawn_async(
    client: Client,
    parent_hierarchy: String,
    agent_full_id: String,
    target_tag: Option<String>,
) -> Result<Vec<DrainedPrompt>, Error> {
    tokio::task::spawn_blocking(move || {
        drain_for_spawn(
            &client,
            &parent_hierarchy,
            &agent_full_id,
            target_tag.as_deref(),
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn drain_for_spawn(
    client: &Client,
    parent_hierarchy: &str,
    agent_full_id: &str,
    target_tag: Option<&str>,
) -> Result<Vec<DrainedPrompt>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    // Two-rule predicate (see plan):
    //  1. ?3 is some AND p.agent_tag = ?3 — the explicit binding tag.
    //  2. p.agent_tag → PENDING tag with (?1, ?2) match.
    // No direct-hierarchy rule: queue/add Direct mode rejects
    // targets without prior completions, so no Direct row can pre-
    // exist for an unspawned agent.
    let rows = collect_matching_prompts(
        &tx,
        "( \
             ?3 IS NOT NULL \
             AND p.agent_tag = ?3 \
         ) \
         OR ( \
             p.agent_tag IS NOT NULL \
             AND EXISTS ( \
                 SELECT 1 FROM tags t \
                 WHERE t.name = p.agent_tag \
                   AND t.parent_agent_instance_hierarchy = ?1 \
                   AND t.agent_full_id = ?2 \
             ) \
         )",
        params![parent_hierarchy, agent_full_id, target_tag],
    )?;
    let drained = reconstruct_and_delete(&tx, rows)?;
    tx.commit()?;
    Ok(drained)
}

/// Row data the drain SELECT pulls out of `prompts`. Threaded into
/// each [`DrainedPrompt`] so re-enqueue can recreate the original
/// row exactly (minus the auto-incremented id, which is rebuilt).
struct DrainedRow {
    prompt_id: i64,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    key: Option<String>,
    enqueued_at: i64,
    prompt_json: String,
}

/// Run the SELECT inside `tx` and return matching rows ordered
/// oldest-first. Shared by both drain predicates; only the
/// `where_clause` differs.
fn collect_matching_prompts(
    tx: &rusqlite::Transaction<'_>,
    where_clause: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<DrainedRow>, Error> {
    let sql = format!(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                p.key, \
                p.enqueued_at, \
                p.prompt \
         FROM prompts p \
         WHERE {where_clause} \
         ORDER BY p.id ASC"
    );
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt
        .query_map(params, |r| {
            Ok(DrainedRow {
                prompt_id: r.get(0)?,
                agent_instance_hierarchy: r.get(1)?,
                agent_tag: r.get(2)?,
                key: r.get(3)?,
                enqueued_at: r.get(4)?,
                prompt_json: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// For each matched row, decode the JSON column as
/// `ResponseContent`, reconstruct a `RichContent` from the
/// referenced per-kind rows, then DELETE the prompt row (which
/// cascades to its content rows via the FK chain). Returns the
/// reconstructed prompts in input (oldest-first) order, each
/// carrying enough metadata for [`re_enqueue_async`] to recreate
/// the original row.
fn reconstruct_and_delete(
    tx: &rusqlite::Transaction<'_>,
    rows: Vec<DrainedRow>,
) -> Result<Vec<DrainedPrompt>, Error> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let response_content: ResponseContent = if row.prompt_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&row.prompt_json).map_err(Error::Json)?
        };
        let content = reconstruct_rich_content(tx, response_content)?;
        // ON DELETE CASCADE on prompt_contents.prompt_id sweeps the
        // master + per-kind rows. PRAGMA foreign_keys = ON is set
        // at connection open (see super::tags::connection).
        tx.execute(
            "DELETE FROM prompts WHERE id = ?1",
            params![row.prompt_id],
        )?;
        out.push(DrainedPrompt {
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
/// (The drain join helper in the CLI handlers reapplies the
/// separator logic on top of these.)
fn reconstruct_rich_content(
    tx: &rusqlite::Transaction<'_>,
    rc: ResponseContent,
) -> Result<RichContent, Error> {
    let ids: Vec<i64> = match rc {
        ResponseContent::One(id) => vec![id],
        ResponseContent::Many(ids) => ids,
    };
    let mut parts: Vec<RichContentPart> = Vec::with_capacity(ids.len());
    for id in ids {
        let row = read_content_with_conn(tx, id)?.ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("queue prompt referenced missing prompt_contents id {id}"),
            ))
        })?;
        parts.push(content_row_to_part(row));
    }
    // Collapse single-text-part to RichContent::Text (lossless).
    // Multi-part — even if all-text — stays RichContent::Parts so
    // the join helper at the call site can insert separators
    // between distinct queue items without losing structure.
    if parts.len() == 1 {
        if let RichContentPart::Text { text } = &parts[0] {
            return Ok(RichContent::Text(text.clone()));
        }
    }
    Ok(RichContent::Parts(parts))
}

// ---------------------------------------------------------------------------
// Re-enqueue — undo a drain when the surrounding spawn/message
// call fails before its first OK stream item. Restores rows with
// their original `enqueued_at` so FIFO ordering is stable across
// the round trip.
// ---------------------------------------------------------------------------

/// Re-INSERT every item in `items` as a fresh `prompts` row + its
/// content rows. Empty `items` short-circuits to `Ok(())` without
/// touching the connection. Everything runs inside one transaction
/// — any failure rolls every restored row back, leaving no orphans.
///
/// `prompts.id` is auto-incremented anew (the original id is gone
/// after the drain DELETE); `enqueued_at` is preserved verbatim
/// from each [`DrainedPrompt`] so subsequent drains find these
/// rows in their original FIFO order.
pub async fn re_enqueue_async(
    client: Client,
    items: Vec<DrainedPrompt>,
) -> Result<(), Error> {
    if items.is_empty() {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || re_enqueue(&client, items))
        .await
        .map_err(spawn_blocking_join_err)?
}

fn re_enqueue(client: &Client, items: Vec<DrainedPrompt>) -> Result<(), Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    for item in items {
        enqueue_with_content_in_tx(
            &tx,
            item.agent_instance_hierarchy.as_deref(),
            item.agent_tag.as_deref(),
            item.key.as_deref(),
            item.enqueued_at,
            item.content,
        )?;
    }
    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Delete-by-id — manually drop a queued prompt without firing
// `agents spawn` / `agents message`. Powers `agents message-queue
// delete <id>`.
// ---------------------------------------------------------------------------

/// Atomically delete the `prompts` row with the given `id` and
/// return its reconstructed shape (same `DrainedPrompt` the drain
/// helpers produce). `None` when no row matches the id. Cascade
/// on `prompt_contents.prompt_id` sweeps every per-kind content
/// row inside the same transaction.
pub async fn delete_by_id_async(
    client: Client,
    id: i64,
) -> Result<Option<DrainedPrompt>, Error> {
    tokio::task::spawn_blocking(move || delete_by_id(&client, id))
        .await
        .map_err(spawn_blocking_join_err)?
}

fn delete_by_id(client: &Client, id: i64) -> Result<Option<DrainedPrompt>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    // Reuse the shared SELECT-and-DELETE pair the drain helpers
    // use; the only difference is the single-row WHERE predicate.
    let rows = collect_matching_prompts(&tx, "p.id = ?1", params![id])?;
    let mut items = reconstruct_and_delete(&tx, rows)?;
    tx.commit()?;
    Ok(items.pop())
}

// ---------------------------------------------------------------------------
// Read-without-delete + clear-by-ids — the API-driven split of the
// drain. The API server polls the client's queue via the
// `read_message_queue` server request, processes the entries upstream,
// and then sends `clear_message_queue` with the entry ids it wants
// released. Mirrors the predicate `drain_for_message` uses (direct
// hierarchy hits + BOUND-tag rule), minus the optional explicit-tag
// rule — the API addresses by hierarchy only.
// ---------------------------------------------------------------------------

/// Non-destructive read of every queue row in scope for an agent,
/// oldest-first (by `prompts.id ASC`, which is equivalent to
/// `enqueued_at` ascending under AUTOINCREMENT). Three-rule
/// predicate:
///
/// 1. Direct: `prompts.agent_instance_hierarchy = target_hierarchy`
/// 2. BOUND tag: `prompts.agent_tag` resolves to a tag with
///    `agent_instance_hierarchy = target_hierarchy`
/// 3. PENDING tag: `prompts.agent_tag` resolves to a tag with
///    `(parent_agent_instance_hierarchy, agent_full_id)` matching
///    the supplied scope — picks up rows enqueued against a tag
///    whose spawn this agent is.
///
/// Returns `(prompts.id, RichContent)` pairs. The caller is
/// responsible for issuing a follow-up [`clear_by_ids_async`] with
/// the ids it wants released; rows left behind remain visible to
/// the next read. `parent_hierarchy` is `""` for rootless agents
/// (matches how `tags::upgrade` stores the parent of a rootless
/// PENDING).
pub async fn read_for_message_async(
    client: Client,
    target_hierarchy: String,
    parent_hierarchy: String,
    agent_full_id: String,
) -> Result<Vec<(i64, RichContent)>, Error> {
    tokio::task::spawn_blocking(move || {
        read_for_message(
            &client,
            &target_hierarchy,
            &parent_hierarchy,
            &agent_full_id,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn read_for_message(
    client: &Client,
    target_hierarchy: &str,
    parent_hierarchy: &str,
    agent_full_id: &str,
) -> Result<Vec<(i64, RichContent)>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    let rows = collect_matching_prompts(
        &tx,
        "p.agent_instance_hierarchy = ?1 \
         OR ( \
             p.agent_tag IS NOT NULL \
             AND EXISTS ( \
                 SELECT 1 FROM tags t \
                 WHERE t.name = p.agent_tag \
                   AND t.agent_instance_hierarchy = ?1 \
             ) \
         ) \
         OR ( \
             p.agent_tag IS NOT NULL \
             AND EXISTS ( \
                 SELECT 1 FROM tags t \
                 WHERE t.name = p.agent_tag \
                   AND t.parent_agent_instance_hierarchy = ?2 \
                   AND t.agent_full_id = ?3 \
             ) \
         )",
        params![target_hierarchy, parent_hierarchy, agent_full_id],
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let rc: ResponseContent = if row.prompt_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&row.prompt_json).map_err(Error::Json)?
        };
        let content = reconstruct_rich_content(&tx, rc)?;
        out.push((row.prompt_id, content));
    }
    // Read-only transaction. Dropping `tx` without commit rolls back
    // (a no-op for pure SELECTs).
    Ok(out)
}

/// Bulk-delete prompt rows by id, scoped to the same three-rule
/// predicate [`read_for_message_async`] uses. Ids outside the scope
/// are silently absorbed (a `DELETE WHERE id = ?` with no match
/// returns 0 rows affected without erroring), which protects against
/// an API caller mis-addressing a row from a different agent's
/// queue.
///
/// Unknown ids are also silently ignored. Empty `ids` short-circuits
/// without touching the connection. `ON DELETE CASCADE` on
/// `prompt_contents.prompt_id` sweeps the per-kind content rows.
pub async fn clear_by_ids_async(
    client: Client,
    agent_instance_hierarchy: String,
    parent_hierarchy: String,
    agent_full_id: String,
    ids: Vec<i64>,
) -> Result<(), Error> {
    if ids.is_empty() {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || {
        clear_by_ids(
            &client,
            &agent_instance_hierarchy,
            &parent_hierarchy,
            &agent_full_id,
            &ids,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn clear_by_ids(
    client: &Client,
    agent_instance_hierarchy: &str,
    parent_hierarchy: &str,
    agent_full_id: &str,
    ids: &[i64],
) -> Result<(), Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    // rusqlite doesn't bind `IN (?)` lists from a slice — loop a
    // prepared single-id DELETE inside one transaction so the whole
    // batch is atomic. The scope predicate matches
    // `read_for_message`'s three rules so the API can't accidentally
    // clear rows from a different agent's queue.
    {
        let mut stmt = tx.prepare(
            "DELETE FROM prompts \
             WHERE id = ?1 \
               AND ( \
                 agent_instance_hierarchy = ?2 \
                 OR ( \
                     agent_tag IS NOT NULL \
                     AND EXISTS ( \
                         SELECT 1 FROM tags t \
                         WHERE t.name = agent_tag \
                           AND t.agent_instance_hierarchy = ?2 \
                     ) \
                 ) \
                 OR ( \
                     agent_tag IS NOT NULL \
                     AND EXISTS ( \
                         SELECT 1 FROM tags t \
                         WHERE t.name = agent_tag \
                           AND t.parent_agent_instance_hierarchy = ?3 \
                           AND t.agent_full_id = ?4 \
                     ) \
                 ) \
               )",
        )?;
        for id in ids {
            stmt.execute(params![
                *id,
                agent_instance_hierarchy,
                parent_hierarchy,
                agent_full_id,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Delivery enumeration — list every distinct `(resolved hierarchy,
// agent_tag)` pair with pending queue rows under (or at) a parent
// hierarchy. Powers `agents message-queue deliver`, which fans out
// one `agents message` call per returned target in parallel.
// ---------------------------------------------------------------------------

/// One addressed delivery target. `agent_instance_hierarchy` is the
/// resolved hierarchy the row would be delivered to (either the
/// row's own hierarchy for Direct rows, or the BOUND tag's bound
/// hierarchy for Tag rows). `agent_tag` is `Some` when the row was
/// originally Tag-addressed — the deliver leaf passes it through
/// to `agents message` as the binding-tag side effect, and
/// surfaces it as the response item's attribution.
#[derive(Debug, Clone)]
pub struct DeliveryTarget {
    pub agent_instance_hierarchy: String,
    pub agent_tag: Option<String>,
}

/// Enumerate every distinct `(resolved hierarchy, agent_tag)` pair
/// with pending queue rows in the subtree rooted at `parent`
/// (inclusive — `parent` itself is in scope). PENDING / ABSENT tag
/// rows are filtered out at the SQL level so the caller never sees
/// addressings that don't resolve to a spawned target.
pub async fn list_delivery_targets_async(
    client: Client,
    parent: String,
) -> Result<Vec<DeliveryTarget>, Error> {
    tokio::task::spawn_blocking(move || list_delivery_targets(&client, &parent))
        .await
        .map_err(spawn_blocking_join_err)?
}

fn list_delivery_targets(
    client: &Client,
    parent: &str,
) -> Result<Vec<DeliveryTarget>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    // Single round-trip. The LEFT JOIN's
    // `t.agent_instance_hierarchy IS NOT NULL` predicate filters
    // PENDING / ABSENT tag rows. DISTINCT collapses true duplicates
    // (multiple rows addressing the same `(hierarchy, tag)` pair).
    let mut stmt = conn.prepare_cached(
        "SELECT DISTINCT \
                COALESCE(t.agent_instance_hierarchy, p.agent_instance_hierarchy) AS hier, \
                p.agent_tag \
         FROM prompts p \
         LEFT JOIN tags t \
             ON p.agent_tag = t.name \
             AND t.agent_instance_hierarchy IS NOT NULL \
         WHERE \
             /* Direct row: target hierarchy in subtree (inclusive). */ \
             ( \
                 p.agent_instance_hierarchy IS NOT NULL \
                 AND ( \
                     p.agent_instance_hierarchy = ?1 \
                     OR p.agent_instance_hierarchy LIKE (?1 || '/%') \
                 ) \
             ) \
             OR \
             /* Tag row resolves through a BOUND tag in the subtree. */ \
             ( \
                 p.agent_tag IS NOT NULL \
                 AND t.agent_instance_hierarchy IS NOT NULL \
                 AND ( \
                     t.agent_instance_hierarchy = ?1 \
                     OR t.agent_instance_hierarchy LIKE (?1 || '/%') \
                 ) \
             ) \
         ORDER BY hier, p.agent_tag",
    )?;
    let rows = stmt
        .query_map([parent], |r| {
            Ok(DeliveryTarget {
                agent_instance_hierarchy: r.get::<_, String>(0)?,
                agent_tag: r.get::<_, Option<String>>(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn spawn_blocking_join_err(e: tokio::task::JoinError) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_client() -> (Client, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let client = Client::new(
            Some(tmp.path().to_path_buf()),
            Some("test"),
            Some("test@test"),
        );
        (client, tmp)
    }

    #[test]
    fn insert_direct_row_returns_id_starting_at_one() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, Some("root/A/inst-1"), None, "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_tag_row_returns_id() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, None, Some("foo"), "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_with_both_targets_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, Some("root/A"), Some("foo"), "[]");
        assert!(err.is_err(), "CHECK constraint must reject both columns set");
    }

    #[test]
    fn insert_with_neither_target_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, None, None, "[]");
        assert!(err.is_err(), "CHECK constraint must reject neither column set");
    }

    #[test]
    fn ids_increment_across_inserts() {
        let (c, _tmp) = fresh_client();
        let a = insert(&c, Some("root/A/h1"), None, "[]").unwrap();
        let b = insert(&c, None, Some("t"), "[]").unwrap();
        let c2 = insert(&c, Some("root/A/h2"), None, "[]").unwrap();
        assert!(a < b && b < c2);
    }
}
