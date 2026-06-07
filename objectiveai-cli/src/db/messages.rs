//! Shared per-agent-id API for the `messages` table.
//!
//! Concurrency note: per-agent state (next-index counter, request-once
//! flag, path-dedup set) is serialized through a per-agent
//! `std::sync::Mutex`. The pool itself is concurrency-safe, so
//! concurrent callers no longer serialize through one
//! `Mutex<Connection>` like the sqlite predecessor did — only callers
//! touching the SAME agent's state contend on that agent's mutex.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;

use super::pending::PendingNotification;
use super::schema::MessageRow;
use super::{Error, Pool};

#[derive(Clone)]
pub struct Queue {
    inner: Arc<QueueInner>,
}

struct QueueInner {
    pool: Pool,
    logs_dir: PathBuf,
    agents: StdMutex<HashMap<String, Arc<AgentEntry>>>,
}

struct AgentEntry {
    state: StdMutex<AgentMutableState>,
}

#[allow(dead_code)]
struct AgentMutableState {
    next_index: u64,
    request_inserted: bool,
    inserted_paths: HashSet<(RequestMessageKind, String)>,
}

impl Queue {
    /// Build a Queue backed by the shared postgres pool. `logs_dir` is
    /// still needed for the notification file write.
    pub fn new(pool: Pool, logs_dir: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(QueueInner {
                pool,
                logs_dir: logs_dir.into(),
                agents: StdMutex::new(HashMap::new()),
            }),
        }
    }

    /// Reserve and return the next monotonic db index for an agent.
    pub async fn reserve_index(
        &self,
        _agent_instance_hierarchy: &str,
    ) -> Result<u64, Error> {
        unimplemented!("db::messages::Queue::reserve_index — stage 7")
    }

    /// Insert one row at a caller-given index.
    pub async fn insert(
        &self,
        _agent_instance_hierarchy: &str,
        _response_id: &str,
        _kind: RequestMessageKind,
        _path: String,
        _timestamp: u64,
        _index: u64,
    ) -> Result<(), Error> {
        unimplemented!("db::messages::Queue::insert — stage 7")
    }

    /// Insert the per-stream request row at most once per agent.
    pub async fn insert_request_once(
        &self,
        _agent_instance_hierarchy: &str,
        _response_id: &str,
        _kind: RequestMessageKind,
        _path: String,
        _timestamp: u64,
    ) -> Result<bool, Error> {
        unimplemented!("db::messages::Queue::insert_request_once — stage 7")
    }

    /// Register a `(kind, path)` pair for dedup under `agent_instance_hierarchy`.
    pub async fn register_path(
        &self,
        _agent_instance_hierarchy: &str,
        _kind: RequestMessageKind,
        _path: &str,
    ) -> Result<bool, Error> {
        unimplemented!("db::messages::Queue::register_path — stage 7")
    }

    /// Write a notification's content out as per-leaf files plus a
    /// parent `RichContentLog` envelope, reserve the next index, and
    /// return a [`PendingNotification`] handle.
    pub async fn write_notification(
        &self,
        _agent_instance_hierarchy: &str,
        _response_id: &str,
        _content: &RichContent,
    ) -> Result<PendingNotification, Error> {
        unimplemented!("db::messages::Queue::write_notification — stage 7")
    }

    /// Insert a previously-reserved notification row at its already-
    /// reserved index.
    pub async fn insert_notification(
        &self,
        _notification: PendingNotification,
    ) -> Result<(), Error> {
        unimplemented!("db::messages::Queue::insert_notification — stage 7")
    }

    /// Read every message for `spawned_agent_instance_hierarchy` whose
    /// `index` is strictly greater than `caller_agent_instance_hierarchy`'s
    /// watermark, then upsert the watermark to the max returned index.
    pub async fn read_new_messages(
        &self,
        _caller_agent_instance_hierarchy: &str,
        _spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, Error> {
        unimplemented!("db::messages::Queue::read_new_messages — stage 7")
    }

    /// Read every message row for `spawned_agent_instance_hierarchy`
    /// (no watermark filter), advancing the watermark to the returned
    /// max.
    pub async fn read_all_messages(
        &self,
        _caller_agent_instance_hierarchy: &str,
        _spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, Error> {
        unimplemented!("db::messages::Queue::read_all_messages — stage 7")
    }
}
