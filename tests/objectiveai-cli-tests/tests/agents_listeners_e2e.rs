//! E2E: the daemon's LIVE agents WebSocket endpoints, consumed with
//! the Rust SDK listener clients:
//!
//! - `/agents/instances/list` ([`AgentsInstancesListListener`])
//!   — every known AIH with its live `active` flag (lock-driven:
//!   Activated on spawn's lock acquire, Deactivated on release).
//! - `/agents/instances/{*aih}` ([`AgentsInstancesListener`])
//!   — one agent's conversation (DB snapshot replay, then the live
//!   log-writer tee) plus its full status record (tags, queue,
//!   attachments, active).
//!
//! Agents are the deterministic MOCK model (`upstream: "mock"` with a
//! scripted `calls` list) — no laboratories, no podman. `on_change` /
//! `on_agent_change` callbacks record EVERY applied change into a
//! mutex'd log, so transient states (active: true during a pass) are
//! asserted without polling races.

mod cli_test_util;

use std::sync::{Arc, Mutex};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::spawn::{
    Path as SpawnPath, Request as SpawnReq, RequestDangerousAdvanced,
    ResponseItem as SpawnItem,
};
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::agents_instances_list_listener::{
    AgentStatus, AgentsInstancesListListener,
};
use objectiveai_sdk::cli::agents_instances_listener::{
    AgentRecord, AssistantResponsePart, ConversationBlock, AgentsInstancesListener,
};

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

/// Poll `$cond` (an `await`-ing bool expression re-evaluated each
/// pass) until true, failing after a generous deadline. The hang
/// watchdog only guards active CLI commands — listener waits carry
/// their own bound.
macro_rules! wait_for {
    ($desc:expr, $cond:expr) => {{
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        loop {
            if $cond {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {}",
                $desc
            );
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }};
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// A mock agent with two scripted content-only turns: the first spawn
/// consumes `"hello"`, a wake/second spawn consumes `"world"` (the
/// mock matches satisfied calls from the continuation).
fn two_turn_mock() -> serde_json::Value {
    serde_json::json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [
            { "tool_calls": [], "content": "hello" },
            { "tool_calls": [], "content": "world" }
        ]
    })
}

/// Apply `tag` carrying the mock spec, spawn via the tag with
/// `message`, and return the minted AIH once the pass is fully done.
async fn spawn_via_tag(executor: &Exec, tag: &str, spec: serde_json::Value) -> String {
    let agent_spec =
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(spec)
            .expect("mock agent spec deserializes");
    let _: ApplyResp = cli_test_util::execute_one(
        executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.to_string(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("go".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.to_string(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;
    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    cli_test_util::wait_for_agent(executor, &aih).await;
    aih
}

/// Spawn a SECOND turn on an existing instance (Instance selector on
/// the AIH split), waited to completion.
async fn spawn_second_turn(executor: &Exec, aih: &str, message: &str) {
    let (parent, instance) = aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or((None, aih.to_string()));
    let _: Vec<SpawnItem> = cli_test_util::collect_stream(
        executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple(message.to_string()),
            agent: AgentSelector::Instance {
                parent_agent_instance_hierarchy: parent,
                agent_instance: instance,
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;
    cli_test_util::wait_for_agent(executor, aih).await;
}

/// Every AssistantResponse text across the conversation, in order.
fn assistant_texts(blocks: &[ConversationBlock]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|b| match b {
            ConversationBlock::AssistantResponse { parts, .. } => Some(parts),
            _ => None,
        })
        .flatten()
        .filter_map(|p| match p {
            AssistantResponsePart::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// `/agents/instances/list`: an agent APPEARS active while its spawn
/// pass runs (lock held) and flips inactive — but stays listed — when
/// it completes. The `on_change` recorder captures the transient
/// active state deterministically.
#[tokio::test(flavor = "multi_thread")]
async fn agents_list_stream_activation_lifecycle() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;

    let snapshots: Arc<Mutex<Vec<Vec<AgentStatus>>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&snapshots);
    let listener = AgentsInstancesListListener::new(format!(
        "{addr}/agents/instances/list"
    ))
    .on_change(move |agents| {
        recorder.lock().unwrap().push(agents.to_vec());
    })
    .connect()
    .await
    .expect("connect /agents/instances/list");

    let tag = format!("listeners-agents-{}", nanos());
    let aih = spawn_via_tag(&executor, &tag, two_turn_mock()).await;

    // The recorder saw the agent ACTIVE at some point (the Activated
    // change applied while the lock was held) …
    wait_for!("recorded active snapshot", {
        snapshots
            .lock()
            .unwrap()
            .iter()
            .any(|snap| snap.iter().any(|a| a.agent_instance_hierarchy == aih && a.active))
    });
    // … and the final state lists it INACTIVE (lock released, AIH
    // retained in the set).
    wait_for!("final inactive state", {
        listener
            .agents()
            .await
            .iter()
            .any(|a| a.agent_instance_hierarchy == aih && !a.active)
    });
}

/// `/agents/instances/{*aih}`: the DB snapshot replay (user message +
/// scripted assistant text, then `Live`), the live tee (a second turn
/// appends blocks while connected), the status record's active
/// transitions, and an attachment update — all on one listener.
#[tokio::test(flavor = "multi_thread")]
async fn agent_instance_stream_snapshot_and_live() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;
    // Phase timing (temporary diagnostics): this test fails at a
    // sub-second-identical ~185s across runs — find the fixed ~180s
    // consumer.
    let t0 = std::time::Instant::now();
    macro_rules! mark {
        ($what:expr) => {
            eprintln!("[timing +{:>7.1?}] {}", t0.elapsed(), $what)
        };
    }
    mark!("daemon up");
    // Postmortem watchdog (temporary diagnostics, like the marks): every
    // 5s record daemon lock liveness + TCP accept and log TRANSITIONS —
    // when the ~185s failure fires, this shows whether the daemon died
    // silently (lock released / connect refused) or stayed alive while
    // aborting individual connections.
    {
        let probe_addr = addr
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .to_string();
        let lock_dir = cli_test_util::objectiveai_dir()
            .join("state")
            .join(&state)
            .join("locks");
        let t0p = std::time::Instant::now();
        tokio::spawn(async move {
            let mut last = String::new();
            loop {
                let held =
                    objectiveai_sdk::lockfile::try_held(&lock_dir, "plugins-daemon").await;
                let tcp = match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    tokio::net::TcpStream::connect(&probe_addr),
                )
                .await
                {
                    Ok(Ok(_)) => "tcp-ok",
                    Ok(Err(_)) => "tcp-refused",
                    Err(_) => "tcp-timeout",
                };
                let now = format!("daemon-lock-held={held} {tcp}");
                if now != last {
                    eprintln!("[probe +{:>7.1?}] {now}", t0p.elapsed());
                    last = now;
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }

    // First turn BEFORE connecting: the listener must replay it from
    // the DB snapshot.
    let tag = format!("listeners-aih-{}", nanos());
    let aih = spawn_via_tag(&executor, &tag, two_turn_mock()).await;
    mark!("first turn spawned + settled");

    let records: Arc<Mutex<Vec<AgentRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&records);
    let listener = AgentsInstancesListener::new(format!(
        "{addr}/agents/instances/{aih}"
    ))
    .on_agent_change(move |record| {
        recorder.lock().unwrap().push(record.clone());
    })
    .connect()
    .await
    .expect("connect /agents/instances/{aih}");
    mark!("instance listener connected");

    // Snapshot replay: live marker, the user request, the scripted
    // assistant text, and a settled status record.
    wait_for!("snapshot replay complete", listener.is_live().await);
    mark!("snapshot replay complete");
    let conversation = listener.conversation().await;
    assert!(
        conversation
            .iter()
            .any(|b| matches!(b, ConversationBlock::RequestMessageUser { .. })),
        "replayed conversation carries the user request; got {conversation:#?}"
    );
    assert!(
        assistant_texts(&conversation).iter().any(|t| t.contains("hello")),
        "replayed conversation carries the scripted assistant text; got {:?}",
        assistant_texts(&conversation)
    );
    let record = listener.agent().await.expect("record shipped on connect");
    assert_eq!(record.agent_instance_hierarchy, aih);
    assert!(!record.active, "agent completed before we connected");
    assert!(record.spawned_at.is_some(), "spawned_at from the first row");
    assert!(record.tags.contains(&tag), "spawn tag bound to the AIH");

    // LIVE: a second turn while connected — new blocks arrive over the
    // tee and the record flips active → inactive (both recorded).
    let blocks_before = listener.conversation().await.len();
    mark!("second turn: spawning");
    spawn_second_turn(&executor, &aih, "again").await;
    mark!("second turn spawned + settled");
    wait_for!("second-turn text over the live tee", {
        let conversation = listener.conversation().await;
        conversation.len() > blocks_before
            && assistant_texts(&conversation).iter().any(|t| t.contains("world"))
    });
    wait_for!("recorded active record during the second turn", {
        records.lock().unwrap().iter().any(|r| r.active)
    });
    wait_for!("record settles inactive", {
        listener.agent().await.is_some_and(|r| !r.active)
    });
    mark!("record settled inactive");

    // Attachment update: attach a (DB-only) laboratory id to the AIH —
    // the record rebuild rides the attachments NOTIFY.
    let lab = format!("listeners-attached-lab-{}", nanos());
    let (parent, instance) = aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .unwrap_or((None, aih.clone()));
    let _: AttachResp = cli_test_util::execute_one(
        &executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Instance {
                parent_agent_instance_hierarchy: parent,
                agent_instance: instance,
            },
            laboratory_id: lab.clone(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    mark!("attach returned");
    wait_for!("attached laboratory on the record", {
        listener
            .agent()
            .await
            .is_some_and(|r| r.attached_laboratories.iter().any(|l| l.id == lab))
    });
}
