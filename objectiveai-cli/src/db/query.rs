//! `objectiveai db query` — read-only SQL executor.
//!
//! Wraps the user's single statement in a short-lived read-only
//! transaction with a server-side `statement_timeout` and a
//! client-side `tokio::time::timeout`. The server timeout is set
//! to `requested - 100ms` so Postgres always wins the race and
//! cleans up its own state; if it misses (or the budget is
//! sub-200ms) the Rust-side timeout drops the future and the
//! connection, which lets sqlx return it to the pool.
//!
//! Each result cell is decoded to a `serde_json::Value` by
//! [`pg_value_to_json`], which dispatches on
//! `pg_type.typname`. Common Postgres types (text family,
//! integer family, float family, bool, uuid, timestamps, dates,
//! json/jsonb, bytea, inet/cidr, numeric) get a typed decode;
//! arrays of those common element types recurse; anything else
//! falls back to the text representation if it's available, or
//! a `<unsupported $TYPE>` placeholder.

use crate::error::Error;
use sqlx::{
    Column as _, Row as _, TypeInfo as _, ValueRef as _,
    postgres::{PgRow, PgValueRef},
};
use std::time::Duration;

pub struct RawQueryResult {
    pub command_tag: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, serde::Serialize)]
pub struct Column {
    pub name: String,
    pub r#type: String,
}

pub async fn run_readonly_query(
    pool: &crate::db::Pool,
    sql: &str,
    timeout: Duration,
) -> Result<RawQueryResult, Error> {
    match tokio::time::timeout(timeout, run_inner(pool, sql, timeout)).await {
        Ok(res) => res,
        Err(_) => Err(Error::QueryTimeout),
    }
}

async fn run_inner(
    pool: &crate::db::Pool,
    sql: &str,
    timeout: Duration,
) -> Result<RawQueryResult, Error> {
    let mut tx = pool.begin().await.map_err(crate::db::Error::Sqlx)?;

    let server_ms = timeout
        .saturating_sub(Duration::from_millis(100))
        .as_millis()
        .max(1) as u64;
    sqlx::query("SET LOCAL TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(crate::db::Error::Sqlx)?;
    sqlx::query(&format!("SET LOCAL statement_timeout = {server_ms}"))
        .execute(&mut *tx)
        .await
        .map_err(crate::db::Error::Sqlx)?;
    sqlx::query(&format!("SET LOCAL lock_timeout = {server_ms}"))
        .execute(&mut *tx)
        .await
        .map_err(crate::db::Error::Sqlx)?;

    let rows: Vec<PgRow> = sqlx::query(sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_query_err)?;

    let columns = rows
        .first()
        .map(|r| {
            r.columns()
                .iter()
                .map(|c| Column {
                    name: c.name().to_string(),
                    r#type: c.type_info().name().to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut decoded_rows: Vec<Vec<serde_json::Value>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut cells: Vec<serde_json::Value> = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            let raw = row.try_get_raw(i).map_err(crate::db::Error::Sqlx)?;
            cells.push(pg_value_to_json(raw)?);
        }
        decoded_rows.push(cells);
    }

    let kw = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_uppercase();
    let command_tag = format!("{kw} {}", rows.len());

    tx.commit().await.map_err(crate::db::Error::Sqlx)?;
    Ok(RawQueryResult {
        command_tag,
        columns,
        rows: decoded_rows,
    })
}

fn map_query_err(err: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &err {
        match db.code().as_deref() {
            Some("25006") => return Error::QueryReadOnlyViolation,
            Some("57014") => return Error::QueryTimeout,
            _ => {}
        }
    }
    Error::Db(crate::db::Error::Sqlx(err))
}

fn pg_value_to_json(value: PgValueRef<'_>) -> Result<serde_json::Value, Error> {
    use serde_json::{Number, Value};

    if value.is_null() {
        return Ok(Value::Null);
    }

    let type_name = value.type_info().name().to_string();
    match type_name.as_str() {
        "BOOL" => Ok(Value::Bool(decode::<bool>(value)?)),
        "INT2" => Ok(Value::Number(decode::<i16>(value)?.into())),
        "INT4" => Ok(Value::Number(decode::<i32>(value)?.into())),
        "INT8" => Ok(Value::Number(decode::<i64>(value)?.into())),
        "OID" => {
            let v: sqlx::postgres::types::Oid = decode(value)?;
            Ok(Value::Number(v.0.into()))
        }
        "FLOAT4" => {
            let v: f32 = decode(value)?;
            Ok(Number::from_f64(v as f64)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(v.to_string())))
        }
        "FLOAT8" => {
            let v: f64 = decode(value)?;
            Ok(Number::from_f64(v)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(v.to_string())))
        }
        "NUMERIC" => {
            let s = value.as_str().map_err(|e| {
                Error::Db(crate::db::Error::InvalidData(format!(
                    "numeric decode (text): {e}"
                )))
            })?;
            Ok(Value::String(s.to_string()))
        }
        "TEXT" | "VARCHAR" | "BPCHAR" | "CHAR" | "NAME" | "UNKNOWN" => {
            Ok(Value::String(decode::<String>(value)?))
        }
        "UUID" => {
            let v: sqlx::types::Uuid = decode(value)?;
            Ok(Value::String(v.hyphenated().to_string()))
        }
        "DATE" => {
            let v: chrono::NaiveDate = decode(value)?;
            Ok(Value::String(v.to_string()))
        }
        "TIME" => {
            let v: chrono::NaiveTime = decode(value)?;
            Ok(Value::String(v.to_string()))
        }
        "TIMESTAMP" => {
            let v: chrono::NaiveDateTime = decode(value)?;
            Ok(Value::String(v.format("%Y-%m-%dT%H:%M:%S%.f").to_string()))
        }
        "TIMESTAMPTZ" => {
            let v: chrono::DateTime<chrono::Utc> = decode(value)?;
            Ok(Value::String(v.to_rfc3339()))
        }
        "BYTEA" => {
            use base64::Engine;
            let v: Vec<u8> = decode(value)?;
            Ok(Value::String(
                base64::engine::general_purpose::STANDARD.encode(&v),
            ))
        }
        "JSON" | "JSONB" => {
            let v: sqlx::types::Json<serde_json::Value> = decode(value)?;
            Ok(v.0)
        }
        "INET" | "CIDR" => {
            let v: sqlx::types::ipnetwork::IpNetwork = decode(value)?;
            Ok(Value::String(v.to_string()))
        }
        "TEXT[]" | "VARCHAR[]" | "BPCHAR[]" | "NAME[]" => {
            let v: Vec<String> = decode(value)?;
            Ok(Value::Array(v.into_iter().map(Value::String).collect()))
        }
        "BOOL[]" => {
            let v: Vec<bool> = decode(value)?;
            Ok(Value::Array(v.into_iter().map(Value::Bool).collect()))
        }
        "INT2[]" => {
            let v: Vec<i16> = decode(value)?;
            Ok(Value::Array(
                v.into_iter().map(|n| Value::Number(n.into())).collect(),
            ))
        }
        "INT4[]" => {
            let v: Vec<i32> = decode(value)?;
            Ok(Value::Array(
                v.into_iter().map(|n| Value::Number(n.into())).collect(),
            ))
        }
        "INT8[]" => {
            let v: Vec<i64> = decode(value)?;
            Ok(Value::Array(
                v.into_iter().map(|n| Value::Number(n.into())).collect(),
            ))
        }
        "FLOAT4[]" => {
            let v: Vec<f32> = decode(value)?;
            Ok(Value::Array(
                v.into_iter()
                    .map(|n| {
                        Number::from_f64(n as f64)
                            .map(Value::Number)
                            .unwrap_or_else(|| Value::String(n.to_string()))
                    })
                    .collect(),
            ))
        }
        "FLOAT8[]" => {
            let v: Vec<f64> = decode(value)?;
            Ok(Value::Array(
                v.into_iter()
                    .map(|n| {
                        Number::from_f64(n)
                            .map(Value::Number)
                            .unwrap_or_else(|| Value::String(n.to_string()))
                    })
                    .collect(),
            ))
        }
        "UUID[]" => {
            let v: Vec<sqlx::types::Uuid> = decode(value)?;
            Ok(Value::Array(
                v.into_iter()
                    .map(|u| Value::String(u.hyphenated().to_string()))
                    .collect(),
            ))
        }
        _ => decode_text_fallback(value),
    }
}

fn decode<'a, T>(value: PgValueRef<'a>) -> Result<T, Error>
where
    T: sqlx::Decode<'a, sqlx::Postgres>,
{
    T::decode(value).map_err(|e| {
        Error::Db(crate::db::Error::InvalidData(format!("decode: {e}")))
    })
}

fn decode_text_fallback(value: PgValueRef<'_>) -> Result<serde_json::Value, Error> {
    let type_name = value.type_info().name().to_string();
    match value.as_str() {
        Ok(s) => Ok(serde_json::Value::String(s.to_string())),
        Err(_) => Ok(serde_json::Value::String(format!(
            "<unsupported {type_name}>"
        ))),
    }
}
