//! Lock-heldness repro for the `agents message` wake path — the
//! chat-incident follow-up.
//!
//! `agents message` on an INACTIVE instance wins the AIH family and
//! runs a WAKE pass (empty message; the queue row is the prompt) as a
//! detached IN-DAEMON task — since the daemon split there is no wake
//! subprocess and no `LockClaim::transfer`; agent locks are the
//! daemon's in-process `AgentLockMap`, with NOTHING on disk. The
//! chat-incident evidence showed a second reader running concurrently
//! with a message-spawned wake, so this test pins the invariant
//! nothing else asserts: the AIH family stays CONTINUOUSLY HELD for
//! the wake's whole life, and releases when it ends.
//!
//! The observable is the lock-driven `active` flag on the daemon's
//! `/agents/instances/list` status stream (the same signal
//! `agents_listeners_e2e` pins for spawn): Activated fires on the
//! family acquire, Deactivated on release. The discriminator is EDGE
//! COUNT, not duration — a wake is one fast in-daemon mock turn (tens
//! of milliseconds), so a healthy hold is exactly ONE Activated →
//! Deactivated pair. If the wake ever dropped and re-acquired the
//! family mid-life (the incident's failure mode), a second rising
//! edge appears and the assertion fails.

mod cli_test_util;

use std::sync::{Arc, Mutex};
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
use objectiveai_sdk::cli::websocket_agents_instances_list_listener::WebSocketAgentsInstancesListListener;

const SEED: i64 = 42;

/// Overall cap on the message wait — far beyond any healthy turn;
/// the harness hang-watchdog backstops the whole test anyway.
const MESSAGE_DEADLINE: Duration = Duration::from_secs(180);

/// Poll `$cond` until true, failing after a generous deadline (the
/// hang watchdog only guards active CLI commands — listener waits
/// carry their own bound).
macro_rules! wait_for {
    ($desc:expr, $cond:expr) => {{
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if $cond {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {}",
                $desc
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }};
}

#[tokio::test]
async fn message_wake_child_holds_the_aih_lock() {
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();

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

    // ── 2. Watch the AIH's lock-driven active flag; idle now ─────
    // The LIST route ships typed Activated/Deactivated DELTAS (the
    // instance route rebuilds its record from live state, which races
    // a tens-of-milliseconds wake). Every applied delta fires
    // `on_change` with the folded statuses — record our AIH's flag per
    // event; the sequence of flags carries the edge structure.
    let addr = cli_test_util::daemon_ws_address(&executor, &state).await;
    let transitions: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&transitions);
    let watch_aih = aih.clone();
    let listener = WebSocketAgentsInstancesListListener::new(format!(
        "{addr}/agents/instances/list"
    ))
    .on_change(move |agents| {
        if let Some(status) = agents
            .iter()
            .find(|a| a.agent_instance_hierarchy == watch_aih)
        {
            recorder.lock().unwrap().push(status.active);
        }
    })
    .connect()
    .await
    .expect("connect /agents/instances/list");
    wait_for!("the connect-time list snapshot", {
        listener
            .agents()
            .await
            .iter()
            .any(|a| a.agent_instance_hierarchy == aih)
    });
    assert!(
        listener
            .agents()
            .await
            .iter()
            .any(|a| a.agent_instance_hierarchy == aih && !a.active),
        "agent exited — it must be inactive before the message"
    );

    // ── 3. Message the inactive instance (the wake path) ─────────
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
    while !message_task.is_finished() {
        assert!(
            started.elapsed() < MESSAGE_DEADLINE,
            "agents message did not resolve within {MESSAGE_DEADLINE:?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let response = message_task.await.expect("message task must not panic");
    assert!(
        matches!(response, MessageResponse::Delivered),
        "an instance message resolves Delivered (row consumed), got {response:?}"
    );

    // ── 4. Released once the wake ends ───────────────────────────
    cli_test_util::wait_for_agent(&executor, &aih).await;
    wait_for!("the wake's Activated then Deactivated deltas", {
        let log = transitions.lock().unwrap().clone();
        log.iter().any(|active| *active)
            && log.last().is_some_and(|active| !*active)
    });

    // ── 5. Exactly ONE contiguous active span — hold continuity ──
    // Post-split a wake is one fast in-daemon mock turn (tens of ms),
    // so span DURATION proves nothing; the invariant is that the
    // family is acquired ONCE and held to the end. A drop-and-
    // reacquire mid-life shows up as a second rising edge in the
    // recorded transitions. Re-shipped statuses with an unchanged
    // flag (other agents' deltas) don't edge.
    let log = transitions.lock().unwrap().clone();
    let mut rising_edges = 0usize;
    let mut in_span = false;
    for active in &log {
        match (active, in_span) {
            (true, false) => {
                rising_edges += 1;
                in_span = true;
            }
            (false, true) => in_span = false,
            _ => {}
        }
    }
    assert!(
        !in_span,
        "the record settled inactive above, so the last span must be closed"
    );
    assert!(
        rising_edges >= 1,
        "the wake must activate the agent (no Activated record seen)"
    );
    assert_eq!(
        rising_edges, 1,
        "the AIH family must be acquired exactly once for the wake's \
         whole life — {rising_edges} rising edges means it was dropped \
         and re-acquired mid-life (the chat-incident failure mode)"
    );
}
