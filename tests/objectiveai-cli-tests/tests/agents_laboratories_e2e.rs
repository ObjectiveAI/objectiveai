//! E2E: `agents laboratories attach / detach / list`.
//!
//! No agent spawn needed — these just record/read attachments (no
//! locking: attach/detach work at any time, active agents included).
//! We attach laboratory ids to instance and tag targets, list them
//! back (created_at order), detach, and exercise the error variants
//! (duplicate, not-attached, ref-target).

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::agents::laboratories::detach::{
    Path as DetachPath, Request as DetachReq, Response as DetachResp,
};
use objectiveai_sdk::cli::command::agents::laboratories::list::{
    Path as ListPath, Request as ListReq, ResponseItem as ListItem,
};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::{CommandExecutor, CommandRequest, CommandResponse};

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

fn instance(name: &str) -> AgentSelector {
    AgentSelector::Instance {
        parent_agent_instance_hierarchy: None,
        agent_instance: name.to_string(),
    }
}

async fn attach(executor: &Exec, selector: AgentSelector, lab: &str) {
    let _: AttachResp = cli_test_util::execute_one(
        executor,
        AttachReq {
            path_type: AttachPath::AgentsLaboratoriesAttach,
            selector,
            laboratory_id: lab.to_string(),
            base: Default::default(),
        },
    )
    .await;
}

async fn detach(executor: &Exec, selector: AgentSelector, lab: &str) {
    let _: DetachResp = cli_test_util::execute_one(
        executor,
        DetachReq {
            path_type: DetachPath::AgentsLaboratoriesDetach,
            selector,
            laboratory_id: lab.to_string(),
            base: Default::default(),
        },
    )
    .await;
}

async fn list_ids(executor: &Exec, selector: AgentSelector) -> Vec<String> {
    let items: Vec<ListItem> = cli_test_util::collect_stream(
        executor,
        ListReq {
            path_type: ListPath::AgentsLaboratoriesList,
            selector,
            base: Default::default(),
        },
    )
    .await;
    items.into_iter().map(|i| i.id).collect()
}

/// Drive the raw stream for `request` and assert SOME item is an `Err`
/// whose debug rendering contains `needle`. Works for both unary and
/// streaming leaves (the root `execute` returns a stream either way).
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

/// Attach two labs to an instance target, list (created_at order),
/// detach one then the other, asserting list shrinks accordingly.
#[tokio::test(flavor = "multi_thread")]
async fn attach_list_detach_instance() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    attach(&executor, instance("inst"), "lab-alpha").await;
    attach(&executor, instance("inst"), "lab-beta").await;

    let ids = list_ids(&executor, instance("inst")).await;
    assert_eq!(ids, vec!["lab-alpha".to_string(), "lab-beta".to_string()]);

    detach(&executor, instance("inst"), "lab-alpha").await;
    let ids = list_ids(&executor, instance("inst")).await;
    assert_eq!(ids, vec!["lab-beta".to_string()]);

    detach(&executor, instance("inst"), "lab-beta").await;
    let ids = list_ids(&executor, instance("inst")).await;
    assert!(ids.is_empty(), "expected no attachments, got {ids:?}");
}

/// Duplicate attach and detach-when-absent are errors.
#[tokio::test(flavor = "multi_thread")]
async fn attach_duplicate_and_detach_missing_error() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    attach(&executor, instance("inst"), "lab-dup").await;
    expect_err_containing::<AttachReq, AttachResp>(
        &executor,
        AttachReq {
            path_type: AttachPath::AgentsLaboratoriesAttach,
            selector: instance("inst"),
            laboratory_id: "lab-dup".to_string(),
            base: Default::default(),
        },
        "already attached",
    )
    .await;

    expect_err_containing::<DetachReq, DetachResp>(
        &executor,
        DetachReq {
            path_type: DetachPath::AgentsLaboratoriesDetach,
            selector: instance("inst"),
            laboratory_id: "never-attached".to_string(),
            base: Default::default(),
        },
        "is not attached",
    )
    .await;
}

/// A `Ref` selector can't be attached/detached/listed against — it has
/// no tag/AIH to key the attachment on.
#[tokio::test(flavor = "multi_thread")]
async fn ref_target_is_rejected() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        serde_json::json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "hi"
        }),
    )
    .expect("mock agent spec deserializes");
    let ref_selector = || AgentSelector::Ref {
        agent: AgentRef::Resolved(spec.clone()),
    };

    expect_err_containing::<AttachReq, AttachResp>(
        &executor,
        AttachReq {
            path_type: AttachPath::AgentsLaboratoriesAttach,
            selector: ref_selector(),
            laboratory_id: "lab-x".to_string(),
            base: Default::default(),
        },
        "agent ref",
    )
    .await;

    expect_err_containing::<ListReq, ListItem>(
        &executor,
        ListReq {
            path_type: ListPath::AgentsLaboratoriesList,
            selector: ref_selector(),
            base: Default::default(),
        },
        "agent ref",
    )
    .await;
}

/// Attach to a GROUPED tag target (exercises the tag-resolution
/// path), list, detach.
#[tokio::test(flavor = "multi_thread")]
async fn attach_to_tag() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Create a GROUPED tag (`Target::Agent` makes a fresh tag_group).
    let spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        serde_json::json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "hi"
        }),
    )
    .expect("mock agent spec deserializes");
    let _: ApplyResp = cli_test_util::execute_one(
        &executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: "lab-tag".to_string(),
            target: ApplyTarget::Agent {
                agent_spec: spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;

    let tag = || AgentSelector::Tag {
        agent_tag: "lab-tag".to_string(),
    };
    attach(&executor, tag(), "lab-on-tag").await;
    let ids = list_ids(&executor, tag()).await;
    assert_eq!(ids, vec!["lab-on-tag".to_string()]);

    detach(&executor, tag(), "lab-on-tag").await;
    let ids = list_ids(&executor, tag()).await;
    assert!(ids.is_empty(), "expected no attachments, got {ids:?}");
}
