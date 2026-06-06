//! Deferred-prompt storage for `agents queue {add, list, read id}`.
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
use objectiveai_sdk::cli::command::agents::queue::list::{
    LookupState, ResponseItem,
};
use objectiveai_sdk::cli::command::agents::read::all::ResponseContent;
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
/// variant. Shared by `agents queue read id`'s handler (single-id
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
    content: RichContent,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        enqueue_with_content(
            &client,
            agent_instance_hierarchy.as_deref(),
            agent_tag.as_deref(),
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
    content: RichContent,
) -> Result<i64, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let tx = conn.unchecked_transaction()?;
    // Empty `prompt` placeholder — overwritten by the final UPDATE
    // once we know the id-referenced shape. `prompts.prompt` is
    // NOT NULL so we need *some* value here.
    let prompt_id: i64 = tx.query_row(
        "INSERT INTO prompts (agent_instance_hierarchy, agent_tag, prompt, enqueued_at) \
         VALUES (?1, ?2, '', ?3) \
         RETURNING id",
        params![agent_instance_hierarchy, agent_tag, now_seconds()],
        |r| r.get(0),
    )?;
    let response_content = walk_rich(&tx, prompt_id, content)?;
    let json = serde_json::to_string(&response_content).map_err(Error::Json)?;
    tx.execute(
        "UPDATE prompts SET prompt = ?1 WHERE id = ?2",
        params![json, prompt_id],
    )?;
    tx.commit()?;
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
/// fan out to `agents queue read id` per piece without re-fetching
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
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (
        id,
        agent_instance_hierarchy,
        agent_tag,
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
// payload before the API call fires.
// ---------------------------------------------------------------------------

/// Drain rows targeting `target_hierarchy`, `target_tag` (if some),
/// or any BOUND tag whose `agent_instance_hierarchy` equals
/// `target_hierarchy`. Returns reconstructed `RichContent`s in
/// enqueue (oldest-first) order; deletes the matched rows in the
/// same transaction.
pub async fn drain_for_message_async(
    client: Client,
    target_hierarchy: String,
    target_tag: Option<String>,
) -> Result<Vec<RichContent>, Error> {
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
) -> Result<Vec<RichContent>, Error> {
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
) -> Result<Vec<RichContent>, Error> {
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
) -> Result<Vec<RichContent>, Error> {
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

/// Run the SELECT inside `tx` and return `(prompt_id, prompt_json)`
/// pairs ordered oldest-first. Shared by both drain predicates;
/// only the `where_clause` differs.
fn collect_matching_prompts(
    tx: &rusqlite::Transaction<'_>,
    where_clause: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<(i64, String)>, Error> {
    let sql = format!(
        "SELECT p.id, p.prompt \
         FROM prompts p \
         WHERE {where_clause} \
         ORDER BY p.id ASC"
    );
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt
        .query_map(params, |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// For each matched `(prompt_id, prompt_json)`, decode the JSON
/// column as `ResponseContent`, reconstruct a `RichContent` from
/// the referenced per-kind rows, then DELETE the prompt row (which
/// cascades to its content rows via the FK chain). Returns the
/// reconstructed `RichContent`s in input order.
fn reconstruct_and_delete(
    tx: &rusqlite::Transaction<'_>,
    rows: Vec<(i64, String)>,
) -> Result<Vec<RichContent>, Error> {
    let mut out = Vec::with_capacity(rows.len());
    for (prompt_id, prompt_json) in rows {
        let response_content: ResponseContent = if prompt_json.is_empty() {
            ResponseContent::Many(Vec::new())
        } else {
            serde_json::from_str(&prompt_json).map_err(Error::Json)?
        };
        out.push(reconstruct_rich_content(tx, response_content)?);
        // ON DELETE CASCADE on prompt_contents.prompt_id sweeps the
        // master + per-kind rows. PRAGMA foreign_keys = ON is set
        // at connection open (see super::tags::connection).
        tx.execute("DELETE FROM prompts WHERE id = ?1", params![prompt_id])?;
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
