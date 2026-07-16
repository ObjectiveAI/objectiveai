//! E2E: `agents tags remove` — delete a tag registration by name.
//!
//! Covers the full decision surface: removing a BOUND tag (echoes the
//! vacated hierarchy), removing GROUPED tags (the LAST member of a
//! group garbage-collects the `tag_groups` row — asserted through
//! `tag_group_deleted` on the responses AND a direct row-count
//! probe), removal detaching the tag's laboratory attachments in the
//! same transaction (asserted by direct row probes, including that an
//! unrelated instance target's attachment survives), and the
//! missing-tag error.

mod cli_test_util;

use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::agents::tags::remove::{
    Path as RemovePath, Removed, Request as RemoveReq, Response as RemoveResp,
};
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest, CommandResponse};

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

/// Bind `tag` to an instance leaf (BOUND shape).
async fn apply_bound(executor: &Exec, tag: &str, agent_instance: &str) {
    let _: ApplyResp = cli_test_util::execute_one(
        executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.to_string(),
            target: ApplyTarget::AgentInstance {
                agent_instance: agent_instance.to_string(),
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
}

/// Register `tag` as a fresh GROUPED tag carrying a minimal inline
/// agent spec.
async fn apply_grouped(executor: &Exec, tag: &str) {
    let agent_spec = serde_json::from_value(serde_json::json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [{ "tool_calls": [], "content": "hello" }],
    }))
    .expect("inline agent spec parses");
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
}

/// Clone `source`'s resolution under `tag` — a GROUPED source joins
/// the same tag_group (the second member this test's GC assertions
/// need).
async fn apply_joining(executor: &Exec, tag: &str, source: &str) {
    let _: ApplyResp = cli_test_util::execute_one(
        executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.to_string(),
            target: ApplyTarget::AgentTag {
                agent_tag: source.to_string(),
            },
            base: Default::default(),
        },
    )
    .await;
}

async fn remove(executor: &Exec, tag: &str) -> RemoveResp {
    cli_test_util::execute_one(
        executor,
        RemoveReq {
            path_type: RemovePath::AgentsTagsRemove,
            tag: tag.to_string(),
            base: Default::default(),
        },
    )
    .await
}

/// Drive the raw stream for `request` and assert SOME item is an `Err`
/// whose debug rendering contains `needle`.
async fn expect_err_containing<R, T>(executor: &Exec, request: R, needle: &str)
where
    R: CommandRequest + Send + serde::Serialize,
    T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
{
    use futures::StreamExt;
    let stream = executor
        .execute::<R, T>(request, None)
        .await
        .expect("cli execute must start");
    let mut stream = std::pin::pin!(stream);
    let mut saw = false;
    while let Some(item) = stream.next().await {
        if let Err(e) = item {
            if format!("{e:?}").contains(needle) {
                saw = true;
            }
        }
    }
    assert!(saw, "expected an error containing {needle:?}");
}

/// BOUND tag: remove echoes the vacated hierarchy; a second remove is
/// the missing-tag error.
#[tokio::test(flavor = "multi_thread")]
async fn remove_bound_tag_then_missing_error() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    apply_bound(&executor, "rm-bound", "rm-inst").await;
    let resp = remove(&executor, "rm-bound").await;
    assert_eq!(resp.name, "rm-bound");
    match resp.removed {
        Removed::Bound { agent_instance_hierarchy } => {
            assert!(
                agent_instance_hierarchy.ends_with("/rm-inst"),
                "vacated hierarchy echoes the bound target, got {agent_instance_hierarchy:?}"
            );
        }
        other => panic!("expected Bound removal, got {other:?}"),
    }
    assert_eq!(resp.detached_laboratories, 0);

    expect_err_containing::<RemoveReq, RemoveResp>(
        &executor,
        RemoveReq {
            path_type: RemovePath::AgentsTagsRemove,
            tag: "rm-bound".to_string(),
            base: Default::default(),
        },
        "not registered",
    )
    .await;
}

/// GROUPED tags: removing a non-last member keeps the group; removing
/// the last member garbage-collects it — asserted via the responses
/// and a direct `tag_groups` row probe.
#[tokio::test(flavor = "multi_thread")]
async fn remove_grouped_tags_gc_group_on_last() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    apply_grouped(&executor, "rm-grp-a").await;
    apply_joining(&executor, "rm-grp-b", "rm-grp-a").await;

    let groups_before = cli_test_util::db_query(
        &executor,
        "SELECT COUNT(*)::text FROM objectiveai.tag_groups g \
         WHERE EXISTS (SELECT 1 FROM objectiveai.tags t \
                       WHERE t.tag_group = g.id AND t.name IN ('rm-grp-a','rm-grp-b'))",
    )
    .await;
    assert_eq!(groups_before[0][0], serde_json::json!("1"), "one shared group");

    let first = remove(&executor, "rm-grp-a").await;
    match first.removed {
        Removed::Grouped { tag_group_deleted } => {
            assert!(!tag_group_deleted, "b still references the group");
        }
        other => panic!("expected Grouped removal, got {other:?}"),
    }

    let second = remove(&executor, "rm-grp-b").await;
    match second.removed {
        Removed::Grouped { tag_group_deleted } => {
            assert!(tag_group_deleted, "last member GCs the group");
        }
        other => panic!("expected Grouped removal, got {other:?}"),
    }

    let orphans = cli_test_util::db_query(
        &executor,
        "SELECT COUNT(*)::text FROM objectiveai.tag_groups g \
         WHERE NOT EXISTS (SELECT 1 FROM objectiveai.tags t WHERE t.tag_group = g.id)",
    )
    .await;
    assert_eq!(
        orphans[0][0],
        serde_json::json!("0"),
        "no orphaned tag_groups rows after last-member removal"
    );
}

/// Removal detaches the tag's laboratory attachments in the same
/// transaction — and ONLY that tag's (an instance target's attachment
/// survives).
#[tokio::test(flavor = "multi_thread")]
async fn remove_detaches_tag_laboratory_attachments() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    apply_bound(&executor, "rm-attached", "rm-att-inst").await;
    let _: AttachResp = cli_test_util::execute_one(
        &executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Tag {
                agent_tag: "rm-attached".to_string(),
            },
            laboratory_id: "rm-lab-x".to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    let _: AttachResp = cli_test_util::execute_one(
        &executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Instance {
                parent_agent_instance_hierarchy: None,
                agent_instance: "rm-att-other".to_string(),
            },
            laboratory_id: "rm-lab-y".to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;

    let resp = remove(&executor, "rm-attached").await;
    assert_eq!(resp.detached_laboratories, 1, "the tag's one attachment detached");

    let rows = cli_test_util::db_query(
        &executor,
        "SELECT COUNT(*)::text FROM objectiveai.laboratory_attachments WHERE tag = 'rm-attached'",
    )
    .await;
    assert_eq!(rows[0][0], serde_json::json!("0"), "tag attachments gone");
    let survivors = cli_test_util::db_query(
        &executor,
        "SELECT COUNT(*)::text FROM objectiveai.laboratory_attachments \
         WHERE laboratory_id = 'rm-lab-y'",
    )
    .await;
    assert_eq!(
        survivors[0][0],
        serde_json::json!("1"),
        "the instance target's attachment survives"
    );
}
