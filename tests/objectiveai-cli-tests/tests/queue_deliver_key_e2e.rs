//! E2E: `agents queue deliver --key` only wakes targets whose pending
//! deliverable carries one of the given keys.
//!
//! Enqueue 4 messages to 4 distinct tags — two under key "A", two under
//! key "B" — then apply each tag to a DISTINCT mock agent spec (so each
//! is a distinct instance). Deliver with `--key A` (in-process via
//! `stream_spawns` so the spawns finish before we list), then assert
//! `agents instances list` shows exactly the 2 key-A agents — the key-B
//! tags stay GROUPED with pending rows and mint no instance.

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::enqueue::{
    Path as EnqueuePath, Request as EnqueueRequest, Response as EnqueueResponse,
};
use objectiveai_sdk::cli::command::agents::instances::list::{
    Path as InstancesPath, Request as InstancesRequest, ResponseItem as InstancesItem, Target,
};
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::queue::deliver::{
    Path as DeliverPath, Request as DeliverRequest,
    RequestDangerousAdvanced as DeliverDangerousAdvanced, ResponseItem as DeliverItem,
};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyRequest, Response as ApplyResponse, Target as ApplyTarget,
};
use serde_json::json;

/// A distinct mock agent spec — the per-tag-unique `instruction` makes
/// each hash to a distinct `agent_full_id`, hence a distinct instance.
fn mock_spec(label: &str) -> InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    serde_json::from_value(json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": format!("deliver-key agent {label}"),
    }))
    .expect("mock agent spec deserializes")
}

#[tokio::test(flavor = "multi_thread")]
async fn deliver_key_spawns_only_keyed_targets() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // tag -> key. Same key must target DIFFERENT tags to both stay active:
    // the (agent_tag, key) unique-active index soft-flips a same-tag dupe.
    let tags = [
        ("deliver-key-a1", "A"),
        ("deliver-key-a2", "A"),
        ("deliver-key-b1", "B"),
        ("deliver-key-b2", "B"),
    ];

    // 1. Enqueue one message per tag under its key.
    for (tag, key) in tags {
        let _resp: EnqueueResponse = cli_test_util::execute_one(
            &executor,
            EnqueueRequest {
                path_type: EnqueuePath::AgentsEnqueue,
                agent: AgentSelector::Tag {
                    agent_tag: tag.to_string(),
                },
                message: RequestMessage::Simple("ping".to_string()),
                key: Some(key.to_string()),
                base: Default::default(),
            },
        )
        .await;
    }

    // 2. Apply each tag to a DISTINCT mock agent (a GROUPED tag carrying
    //    the spec — no instance is minted until delivery spawns it).
    for (tag, _key) in tags {
        let _resp: ApplyResponse = cli_test_util::execute_one(
            &executor,
            ApplyRequest {
                path_type: ApplyPath::AgentsTagsApply,
                name: tag.to_string(),
                target: ApplyTarget::Agent {
                    agent_spec: mock_spec(tag),
                    parent_agent_instance_hierarchy: None,
                },
                base: Default::default(),
            },
        )
        .await;
    }

    // 3. Deliver only key "A", in-process so the spawns run to completion
    //    before we list (the default detached mode would race). Draining
    //    the stream waits for every spawned agent's turn to finish.
    let _items: Vec<DeliverItem> = cli_test_util::collect_stream(
        &executor,
        DeliverRequest {
            path_type: DeliverPath::AgentsQueueDeliver,
            keys: Some(vec!["A".to_string()]),
            dangerous_advanced: Some(DeliverDangerousAdvanced {
                stream_spawns: Some(true),
            }),
            base: Default::default(),
        },
    )
    .await;

    // 4. Exactly the two key-A agents were spawned.
    let instances: Vec<InstancesItem> = cli_test_util::collect_stream(
        &executor,
        InstancesRequest {
            path_type: InstancesPath::AgentsInstancesList,
            targets: vec![Target::Me],
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(
        instances.len(),
        2,
        "deliver --key A should spawn exactly the 2 key-A agents, got: {instances:?}",
    );
}
