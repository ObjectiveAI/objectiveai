//! Integration tests for `agents logs token-usage get` / `subscribe`.
//!
//! The command reads `objectiveai.agent_token_usage` (a per-AIH
//! most-recent `total_tokens` snapshot the log writer upserts) and, for
//! `subscribe`, races a postgres `LISTEN` (`agent_token_usage_changed`)
//! against the agent's instance-lock release.
//!
//! Most tests drive the command **in-process**: they build an
//! `objectiveai_cli::Context` bound to this test's dir/state (so it
//! resolves the SAME per-state postgres the cli subprocess uses), get a
//! `&Pool`, write token values directly via
//! `db::logs::update_agent_token_usage` (the cli's `db query` command is
//! read-only, so it can't fire the trigger), and call
//! `token_usage::{subscribe,get}::execute(&ctx, req)`. This gives
//! deterministic control of the lock + the trigger/NOTIFY/listener. One
//! test (`token_usage_wired_through_cli_binary`) goes through the real
//! cli binary to prove the SDK/CLI dispatch we added is wired.

mod cli_test_util;

use std::time::Duration;

use futures::StreamExt;

use objectiveai_cli::db::logs::{
    get_agent_token_usage, update_agent_token_usage, wait_for_token_usage_change,
};

use objectiveai_sdk::cli::command::RequestBase;
use objectiveai_sdk::cli::command::agents::logs::token_usage::get as sdk_get;
use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe as sdk_sub;

// cli-side command executors under test.
use objectiveai_cli::command::agents::logs::token_usage::get as cli_get;
use objectiveai_cli::command::agents::logs::token_usage::subscribe as cli_sub;

// ── helpers ────────────────────────────────────────────────────────

/// Build a `Context` bound to this test's `(OBJECTIVEAI_DIR,
/// OBJECTIVEAI_STATE)` and return it. The per-state postgres MUST be
/// spawned through the cli subprocess first (a warmup `db query`):
/// driving `Context::db_handle()`'s spawn flow directly in-process
/// deadlocks, so the in-process `Context` may only *connect* to an
/// already-running cluster, never cold-spawn one. The warmup uses a
/// generous timeout (not the default 30s cap) because a cold cluster
/// starting under the full suite's parallel load can exceed 30s; the
/// subprocess still gets the harness's 120s inactivity hang-guard, so a
/// genuine hang fails fast while a slow-but-progressing spawn survives.
async fn setup() -> objectiveai_cli::context::Context {
    let executor = cli_test_util::executor().await;
    // Cold-spawns (or attaches to) this state's postgres via the proven
    // cli path. 180s absorbs a slow cold start under load.
    let _ = cli_test_util::db_query_with_timeout(&executor, "SELECT 1", 180).await;

    let config = objectiveai_cli::ConfigBuilder {
        objectiveai_dir: Some(cli_test_util::objectiveai_dir().to_string_lossy().into_owned()),
        objectiveai_state: Some(cli_test_util::test_state_name()),
        ..Default::default()
    }
    .build();
    let ctx = objectiveai_cli::context::Context::new(config);
    // The cluster is up now, so this just connects (never cold-spawns).
    ctx.db_client().await.expect("connect to per-state postgres");
    ctx
}

fn sub_request(aih: &str, previous: Option<i64>) -> sdk_sub::Request {
    sdk_sub::Request {
        path_type: sdk_sub::Path::AgentsLogsTokenUsageSubscribe,
        agent_instance_hierarchy: aih.to_string(),
        previous,
        base: RequestBase::default(),
    }
}

fn get_request(aih: &str) -> sdk_get::Request {
    sdk_get::Request {
        path_type: sdk_get::Path::AgentsLogsTokenUsageGet,
        agent_instance_hierarchy: aih.to_string(),
        base: RequestBase::default(),
    }
}

/// Run `subscribe::execute` and drain its (single-item) stream.
async fn run_subscribe(
    ctx: &objectiveai_cli::context::Context,
    req: sdk_sub::Request,
) -> Vec<sdk_sub::ResponseItem> {
    let mut stream = cli_sub::execute(ctx, req)
        .await
        .expect("subscribe execute failed");
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.expect("subscribe stream item was Err"));
    }
    items
}

fn expect_item(items: &[sdk_sub::ResponseItem]) -> &sdk_sub::TokenUsage {
    assert_eq!(items.len(), 1, "subscribe must yield exactly one item");
    match &items[0] {
        sdk_sub::ResponseItem::Item(tu) => tu,
        sdk_sub::ResponseItem::AgentsInactive(_) => {
            panic!("expected an Item, got agents_inactive")
        }
    }
}

// ── tests ──────────────────────────────────────────────────────────

/// The DB mechanism: the `agent_token_usage_changed` trigger + NOTIFY +
/// the `wait_for_token_usage_change` listener, including that a
/// same-value overwrite does NOT wake it (dedup).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn db_layer_change_fires_and_dedups() {
    let ctx = setup().await;
    let pool = ctx.db_client().await.unwrap().clone();
    let aih = "db-layer-aih";

    update_agent_token_usage(&pool, aih, 100).await.unwrap();

    let waiter_pool = pool.clone();
    let waiter_aih = aih.to_string();
    let mut waiter = tokio::spawn(async move {
        wait_for_token_usage_change(&waiter_pool, &waiter_aih, Some(100)).await
    });

    // Same-value overwrite: the trigger fires, but the listener re-reads
    // and sees no change from the baseline, so it must keep waiting.
    update_agent_token_usage(&pool, aih, 100).await.unwrap();
    let still_pending = tokio::time::timeout(Duration::from_millis(300), &mut waiter).await;
    assert!(
        still_pending.is_err(),
        "a same-value overwrite must not wake the waiter"
    );

    // A real change wakes it with the new value.
    update_agent_token_usage(&pool, aih, 250).await.unwrap();
    let woke = waiter.await.unwrap().expect("wait_for_token_usage_change");
    assert_eq!(woke, 250);
    assert_eq!(get_agent_token_usage(&pool, aih).await.unwrap(), Some(250));
}

/// `--previous` fast path: when a value is stored and differs from
/// `previous`, subscribe returns it immediately (no blocking).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subscribe_fast_path_returns_current() {
    let ctx = setup().await;
    let pool = ctx.db_client().await.unwrap().clone();
    let aih = "fast-path-aih";

    update_agent_token_usage(&pool, aih, 42).await.unwrap();

    let items = run_subscribe(&ctx, sub_request(aih, Some(41))).await;
    let tu = expect_item(&items);
    assert_eq!(tu.total_tokens, 42);
    assert_eq!(tu.agent_instance_hierarchy, aih);
}

/// The core subscription behavior: with the instance lock held (so the
/// lock-release arm blocks), a token-value change wakes subscribe with
/// the new value. `previous == baseline` makes it timing-robust (fast
/// path if the change already landed, listener wake otherwise).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subscribe_wakes_on_change_while_locked() {
    let ctx = setup().await;
    let pool = ctx.db_client().await.unwrap().clone();
    let aih = "wake-on-change-aih";

    // Hold the agent's instance lock so `wait_released` parks.
    let (lock_dir, lock_key) = objectiveai_cli::command::agents::locks::agent_instance_lock(
        &ctx.filesystem.state_dir(),
        aih,
    );
    let claim = objectiveai_sdk::lockfile::wait_acquire(&lock_dir, &lock_key, "")
        .await
        .expect("acquire instance lock");

    update_agent_token_usage(&pool, aih, 10).await.unwrap(); // baseline

    let sub_ctx = ctx.clone();
    let handle =
        tokio::spawn(async move { run_subscribe(&sub_ctx, sub_request(aih, Some(10))).await });

    // Change it while subscribe is (or will be) watching.
    update_agent_token_usage(&pool, aih, 77).await.unwrap();

    let items = handle.await.expect("subscribe task join");
    let tu = expect_item(&items);
    assert_eq!(tu.total_tokens, 77);

    claim.release().expect("release instance lock");
}

/// When the instance lock drops with no pending token change, subscribe
/// returns the bare `agents_inactive` signal.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subscribe_agents_inactive_on_lock_release() {
    let ctx = setup().await;
    let aih = "inactive-aih"; // fresh: no agent_token_usage row

    let (lock_dir, lock_key) = objectiveai_cli::command::agents::locks::agent_instance_lock(
        &ctx.filesystem.state_dir(),
        aih,
    );
    let claim = objectiveai_sdk::lockfile::wait_acquire(&lock_dir, &lock_key, "")
        .await
        .expect("acquire instance lock");

    let sub_ctx = ctx.clone();
    let handle =
        tokio::spawn(async move { run_subscribe(&sub_ctx, sub_request(aih, None)).await });

    // Let subscribe reach its blocking `wait_released`, then release with
    // no token change → it must resolve to agents_inactive.
    tokio::time::sleep(Duration::from_millis(150)).await;
    claim.release().expect("release instance lock");

    let items = handle.await.expect("subscribe task join");
    assert_eq!(items.len(), 1);
    assert!(
        matches!(items[0], sdk_sub::ResponseItem::AgentsInactive(_)),
        "expected agents_inactive, got {:?}",
        items[0]
    );
}

/// `get` returns null for an unknown AIH and the stored value once set.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_returns_none_then_some() {
    let ctx = setup().await;
    let pool = ctx.db_client().await.unwrap().clone();

    let unknown = cli_get::execute(&ctx, get_request("get-unknown-aih"))
        .await
        .expect("get execute");
    assert_eq!(unknown.total_tokens, None);

    let aih = "get-some-aih";
    update_agent_token_usage(&pool, aih, 555).await.unwrap();
    let known = cli_get::execute(&ctx, get_request(aih))
        .await
        .expect("get execute");
    assert_eq!(known.total_tokens, Some(555));
    assert_eq!(known.agent_instance_hierarchy, aih);
}

/// End-to-end through the real cli binary: a mock agent completion
/// populates `agent_token_usage` via the log writer, then `get` and
/// `subscribe` (dispatched through the binary) read it back.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_usage_wired_through_cli_binary() {
    use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
    use objectiveai_sdk::cli::command::agents::message::RequestMessage;
    use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
    use objectiveai_sdk::cli::command::agents::spawn::{
        Path as SpawnPath, Request as SpawnRequest, RequestDangerousAdvanced,
        ResponseItem as SpawnResponseItem,
    };

    let executor = cli_test_util::executor().await;

    let spawn_items: Vec<SpawnResponseItem> = cli_test_util::collect_stream(
        &executor,
        SpawnRequest {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("token usage".to_string()),
            agent: AgentSelector::Ref {
                agent: AgentRef::Resolved(
                    serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
                        serde_json::json!({"upstream":"mock","output_mode":"instruction"}),
                    )
                    .expect("inline mock agent must deserialize"),
                ),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(42),
            }),
            base: Default::default(),
        },
    )
    .await;

    let aih = spawn_items
        .iter()
        .find_map(|item| match item {
            SpawnResponseItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn must emit a chunk with a non-empty agent_instance_hierarchy");

    // Ensure the runner finalized (log writer flushed the usage row).
    cli_test_util::wait_for_agent(&executor, &aih).await;

    // `get` through the binary — the mock's terminal usage should have
    // been recorded by the log writer.
    let got: sdk_get::Response =
        cli_test_util::execute_one(&executor, get_request(&aih)).await;
    let n = got
        .total_tokens
        .expect("mock completion must record a token-usage snapshot");
    assert!(n > 0, "total_tokens should be positive, got {n}");

    // `subscribe --previous n-1` through the binary → fast path Item{n}.
    let sub_items: Vec<sdk_sub::ResponseItem> =
        cli_test_util::collect_stream(&executor, sub_request(&aih, Some(n - 1))).await;
    let tu = expect_item(&sub_items);
    assert_eq!(tu.total_tokens, n);
}
