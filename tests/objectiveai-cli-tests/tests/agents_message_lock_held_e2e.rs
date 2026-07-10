//! Lock-heldness repro for the `agents message` wake path — the
//! chat-incident follow-up.
//!
//! `agents message` on an INACTIVE instance wins the AIH family,
//! spawns a WAKE child (empty message; the queue row is the prompt),
//! and TRANSFERS the claim into it (`LockClaim::transfer` — on
//! Windows a `DuplicateHandle` handoff, the parent's handles closed
//! after). The chat-incident evidence showed a second reader running
//! concurrently with a message-spawned child, so this test pins the
//! invariant nothing else asserts: the AIH lock stays CONTINUOUSLY
//! HELD (sdk `lockfile::try_held`, the read-only probe) for the wake
//! child's whole life, and releases when it exits.
//!
//! The discriminator is DURATION, not an instant: a healthy transfer
//! keeps the lock held from the waiter's acquire through the child's
//! multi-second life (process boot via the cargo shim alone is
//! seconds), so the longest observed held-streak spans well over
//! [`MIN_HELD_STREAK`]. If the transfer silently drops the OS lock —
//! the incident's prime suspect — held-ness blips only for the
//! waiter's acquire→spawn window (well under 100ms) and the streak
//! assertion fails.

mod cli_test_util;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::{
    Request as MessageRequest,
    RequestDangerousAdvanced as MessageDangerousAdvanced, RequestMessage,
    Response as MessageResponse,
};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn::{
    Request as SpawnRequest, RequestDangerousAdvanced as SpawnDangerousAdvanced,
    ResponseItem as SpawnResponseItem,
};

const SEED: i64 = 42;

/// The longest held-streak must span at least this long. Healthy:
/// the child holds for its whole life (seconds — the cargo-shim boot
/// alone exceeds this). Broken transfer: only the waiter's
/// acquire→spawn blip (<100ms) is ever observed.
const MIN_HELD_STREAK: Duration = Duration::from_millis(500);

/// Overall cap on the message wait — far beyond any healthy turn;
/// the harness hang-watchdog backstops the whole test anyway.
const MESSAGE_DEADLINE: Duration = Duration::from_secs(180);

/// Mirror of the cli's `agent_instance_lock` split
/// (`objectiveai-cli/src/command/agents/locks.rs`): every hierarchy
/// segment but the last extends the dir, the leaf is the key.
fn instance_lock(state_dir: &Path, hierarchy: &str) -> (PathBuf, String) {
    let mut dir = state_dir.join("locks").join("agents").join("instances");
    let mut segments = hierarchy.split('/').peekable();
    let mut key = String::new();
    while let Some(segment) = segments.next() {
        if segments.peek().is_some() {
            dir.push(segment);
        } else {
            key = segment.to_string();
        }
    }
    (dir, key)
}

#[tokio::test]
async fn message_wake_child_holds_the_aih_lock() {
    let executor = cli_test_util::executor().await;

    // ── 1. Spawn a mock agent and let it finish ──────────────────
    let spawn_request = SpawnRequest {
        path_type: objectiveai_sdk::cli::command::agents::spawn::Path::AgentsSpawn,
        message: RequestMessage::Simple("first turn".to_string()),
        agent: AgentSelector::Ref {
            agent: AgentRef::Resolved(
                serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                    serde_json::json!({"upstream":"mock","output_mode":"instruction"}),
                )
                .expect("inline mock agent must deserialize"),
            ),
        },
        dangerous_advanced: Some(SpawnDangerousAdvanced {
            stream: Some(true),
            seed: Some(SEED),
        }),
        base: Default::default(),
    };
    let spawn_items: Vec<SpawnResponseItem> =
        cli_test_util::collect_stream(&executor, spawn_request).await;
    let aih = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(chunk) if !chunk.agent_instance_hierarchy.is_empty() => {
                Some(chunk.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("agents spawn must emit a Chunk with a non-empty agent_instance_hierarchy");
    cli_test_util::wait_for_agent(&executor, &aih).await;

    // ── 2. The AIH lock coordinates; free while the agent is idle ─
    let state_dir = cli_test_util::objectiveai_dir()
        .join("state")
        .join(cli_test_util::test_state_name());
    let (lock_dir, lock_key) = instance_lock(&state_dir, &aih);
    assert!(
        !objectiveai_sdk::lockfile::try_held(&lock_dir, &lock_key).await,
        "agent exited — its AIH lock must be free before the message"
    );

    // ── 3. Message the inactive instance (the wake path), sampling
    //       held-ness the whole time ─────────────────────────────
    let (parent, instance) = aih
        .rsplit_once('/')
        .map(|(p, i)| (Some(p.to_string()), i.to_string()))
        .expect("aih must carry at least one '/'");
    let message_request = MessageRequest {
        path_type: objectiveai_sdk::cli::command::agents::message::Path::AgentsMessage,
        agent: AgentSelector::Instance {
            parent_agent_instance_hierarchy: parent,
            agent_instance: instance,
        },
        message: RequestMessage::Simple("wake up".to_string()),
        dangerous_advanced: Some(MessageDangerousAdvanced { seed: Some(SEED) }),
        base: Default::default(),
    };
    // Build the message's executor BEFORE spawning the task: the
    // per-test state name comes from the CURRENT THREAD's name, and a
    // runtime worker thread inside `tokio::spawn` has the wrong one.
    let message_executor = cli_test_util::executor().await;
    let message_task = tokio::spawn(async move {
        cli_test_util::execute_one::<_, _, MessageResponse>(
            &message_executor,
            message_request,
        )
        .await
    });

    let started = Instant::now();
    let mut streak_start: Option<Instant> = None;
    let mut longest_streak = Duration::ZERO;
    while !message_task.is_finished() {
        assert!(
            started.elapsed() < MESSAGE_DEADLINE,
            "agents message did not resolve within {MESSAGE_DEADLINE:?}"
        );
        let held = objectiveai_sdk::lockfile::try_held(&lock_dir, &lock_key).await;
        match (held, streak_start) {
            (true, None) => streak_start = Some(Instant::now()),
            (true, Some(start)) => longest_streak = longest_streak.max(start.elapsed()),
            (false, _) => streak_start = None,
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let response = message_task.await.expect("message task must not panic");
    assert!(
        matches!(response, MessageResponse::Delivered),
        "an instance message resolves Delivered (row consumed), got {response:?}"
    );
    assert!(
        longest_streak >= MIN_HELD_STREAK,
        "the AIH lock must stay held for the wake child's whole life \
         (longest observed streak {longest_streak:?} < {MIN_HELD_STREAK:?}) — \
         a sub-100ms blip means the claim transfer dropped the OS lock"
    );

    // ── 4. Released once the wake child exits ────────────────────
    cli_test_util::wait_for_agent(&executor, &aih).await;
    assert!(
        !objectiveai_sdk::lockfile::try_held(&lock_dir, &lock_key).await,
        "the AIH lock must release when the wake child exits"
    );
}
