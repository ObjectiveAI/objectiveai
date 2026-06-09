//! `db query` — pre-flight validate, run, and (optionally)
//! tokenize the response against a per-part budget.
//!
//! Pre-flight rejects shapes the response variant can't represent:
//! - multi-statement input (any unquoted `;` followed by non-empty
//!   non-comment content),
//! - `COPY ... TO STDOUT|STDIN` (no FE/BE protocol path for it),
//! - transaction control verbs (`BEGIN`, `START`, `COMMIT`, `END`,
//!   `ROLLBACK`, `SAVEPOINT`, `RELEASE`) — the handler runs the
//!   user query inside its own read-only tx and these would
//!   collide.
//!
//! Tokenization rule (from the issue): tokenize each part of the
//! response separately via `serde_json::to_string` →
//! `tiktoken-rs` `o200k_base` `encode_with_special_tokens`, sum
//! the per-part counts, and reject if the sum exceeds the cap.
//! "Each part" means `command_tag`, the full serialized
//! `columns` array, and one serialization per row.

use std::time::Duration;

use objectiveai_sdk::cli::command::db::query::{Column, Request, Response};

use crate::context::Context;
use crate::db::query::{Column as RawColumn, RawQueryResult};
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    pre_flight_validate(&request.query)?;

    let timeout = Duration::from_secs(request.timeout_seconds);
    let raw = crate::db::query::run_readonly_query(&ctx.db, &request.query, timeout).await?;

    if let Some(limit) = request.max_tokens {
        let actual = count_tokens(&raw)?;
        if actual > limit {
            return Err(Error::TokenBudgetExceeded { limit, actual });
        }
    }

    let RawQueryResult { command_tag, columns, rows } = raw;
    Ok(Response {
        command_tag,
        columns: columns
            .into_iter()
            .map(|RawColumn { name, r#type }| Column { name, r#type })
            .collect(),
        rows,
        truncated: false,
    })
}

/// Cheap leading-token / non-quoted-semicolon scan. We don't
/// parse SQL — we just reject shapes the response variant can't
/// represent. Anything else is forwarded to Postgres, which will
/// reject writes via the read-only transaction.
fn pre_flight_validate(sql: &str) -> Result<(), Error> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidQuery("empty query".to_string()));
    }

    if has_trailing_statement(sql) {
        return Err(Error::InvalidQuery(
            "only one statement per call".to_string(),
        ));
    }

    let leading = leading_keyword(trimmed);
    match leading.as_str() {
        "COPY" => {
            // COPY ... TO STDOUT / FROM STDIN both need the
            // FE/BE COPY protocol which we don't expose.
            if has_copy_stdin_stdout(trimmed) {
                return Err(Error::InvalidQuery(
                    "COPY ... TO STDOUT / FROM STDIN is not supported".to_string(),
                ));
            }
        }
        "BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "SAVEPOINT" | "RELEASE" => {
            return Err(Error::InvalidQuery(
                "transaction control is not permitted".to_string(),
            ));
        }
        _ => {}
    }

    Ok(())
}

fn leading_keyword(sql: &str) -> String {
    sql.split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
}

/// `true` if there's a non-quoted `;` followed by any non-
/// whitespace, non-comment content. A single trailing `;` after
/// the last statement is fine.
fn has_trailing_statement(sql: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut last_semi: Option<usize> = None;
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == '*' && i + 1 < bytes.len() && bytes[i + 1] as char == '/' {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_single {
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '-' if i + 1 < bytes.len() && bytes[i + 1] as char == '-' => {
                in_line_comment = true;
                i += 2;
                continue;
            }
            '/' if i + 1 < bytes.len() && bytes[i + 1] as char == '*' => {
                in_block_comment = true;
                i += 2;
                continue;
            }
            ';' => last_semi = Some(i),
            _ => {}
        }
        i += 1;
    }
    let Some(idx) = last_semi else { return false };
    sql[idx + 1..].chars().any(|c| !c.is_whitespace())
}

fn has_copy_stdin_stdout(sql: &str) -> bool {
    let upper = sql.to_ascii_uppercase();
    upper.contains("STDIN") || upper.contains("STDOUT")
}

fn count_tokens(raw: &RawQueryResult) -> Result<u64, Error> {
    let enc = tiktoken_rs::o200k_base()
        .map_err(|e| Error::InvalidQuery(format!("tiktoken init: {e}")))?;
    let mut total: u64 = 0;
    total += enc.encode_with_special_tokens(&raw.command_tag).len() as u64;
    let columns_json = serde_json::to_string(&raw.columns).map_err(crate::db::Error::Json)?;
    total += enc.encode_with_special_tokens(&columns_json).len() as u64;
    for row in &raw.rows {
        let row_json = serde_json::to_string(row).map_err(crate::db::Error::Json)?;
        total += enc.encode_with_special_tokens(&row_json).len() as u64;
    }
    Ok(total)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::query as sdk;
    use objectiveai_sdk::cli::command::db::query::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::query as sdk;
    use objectiveai_sdk::cli::command::db::query::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
