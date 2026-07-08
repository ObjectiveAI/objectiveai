//! E2E: `laboratories attach / detach` (+ readback via
//! `agents instances get`).
//!
//! No agent spawn needed — these just record attachments (no locking:
//! attach/detach work at any time, active agents included). There is
//! no attachments list command; the readback surface is the
//! `laboratories` field on `agents instances get` (the effective
//! AIH ∪ bound-tags union), which also carries `attached_at` +
//! `attached_by`. We attach laboratory ids to instance and tag
//! targets and exercise the error variants (duplicate, not-attached,
//! ref-target).

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::instances::get::{
    Path as GetPath, Request as GetReq, ResponseItem as GetItem, Target as GetTarget,
};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::laboratories::detach::{
    Path as DetachPath, Request as DetachReq, Response as DetachResp,
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
            path_type: AttachPath::LaboratoriesAttach,
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
            path_type: DetachPath::LaboratoriesDetach,
            selector,
            laboratory_id: lab.to_string(),
            base: Default::default(),
        },
    )
    .await;
}

/// Read the instance's effective attachments back through
/// `agents instances get` — the only attachments read surface.
async fn get_labs(
    executor: &Exec,
    name: &str,
) -> Vec<objectiveai_sdk::cli::command::agents::instances::list::LaboratoryAttachment> {
    let items: Vec<GetItem> = cli_test_util::collect_stream(
        executor,
        GetReq {
            path_type: GetPath::AgentsInstancesGet,
            targets: vec![GetTarget::Direct {
                parent_agent_instance_hierarchy: None,
                agent_instance: name.to_string(),
            }],
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(items.len(), 1, "expected exactly one get item");
    items
        .into_iter()
        .next()
        .unwrap()
        .laboratories
        .expect("get populates laboratories")
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

/// Attach two labs to an instance target, read them back through
/// `agents instances get` (created_at order, attached_at/attached_by
/// populated), detach one then the other, asserting the set shrinks.
#[tokio::test(flavor = "multi_thread")]
async fn attach_get_detach_instance() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    attach(&executor, instance("inst"), "lab-alpha").await;
    attach(&executor, instance("inst"), "lab-beta").await;

    let labs = get_labs(&executor, "inst").await;
    let ids: Vec<&str> = labs.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, vec!["lab-alpha", "lab-beta"]);
    for lab in &labs {
        assert!(!lab.attached_at.is_empty(), "attached_at populated");
        assert!(lab.attached_by.is_some(), "attached_by recorded");
    }

    detach(&executor, instance("inst"), "lab-alpha").await;
    let labs = get_labs(&executor, "inst").await;
    let ids: Vec<&str> = labs.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, vec!["lab-beta"]);

    detach(&executor, instance("inst"), "lab-beta").await;
    let labs = get_labs(&executor, "inst").await;
    assert!(labs.is_empty(), "expected no attachments, got {labs:?}");
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
            path_type: AttachPath::LaboratoriesAttach,
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
            path_type: DetachPath::LaboratoriesDetach,
            selector: instance("inst"),
            laboratory_id: "never-attached".to_string(),
            base: Default::default(),
        },
        "is not attached",
    )
    .await;
}

/// A `Ref` selector can't be attached/detached against — it has no
/// tag/AIH to key the attachment on.
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
            path_type: AttachPath::LaboratoriesAttach,
            selector: ref_selector(),
            laboratory_id: "lab-x".to_string(),
            base: Default::default(),
        },
        "agent ref",
    )
    .await;

    expect_err_containing::<DetachReq, DetachResp>(
        &executor,
        DetachReq {
            path_type: DetachPath::LaboratoriesDetach,
            selector: ref_selector(),
            laboratory_id: "lab-x".to_string(),
            base: Default::default(),
        },
        "agent ref",
    )
    .await;
}

/// Attach to a GROUPED tag target (exercises the tag-resolution path).
/// A GROUPED tag resolves to no exact AIH, so `agents instances get`
/// can't read it back — the round-trip is verified behaviorally: a
/// duplicate attach errors (the row exists), detach succeeds, and a
/// second detach errors (the row is gone).
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
    expect_err_containing::<AttachReq, AttachResp>(
        &executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: tag(),
            laboratory_id: "lab-on-tag".to_string(),
            base: Default::default(),
        },
        "already attached",
    )
    .await;

    detach(&executor, tag(), "lab-on-tag").await;
    expect_err_containing::<DetachReq, DetachResp>(
        &executor,
        DetachReq {
            path_type: DetachPath::LaboratoriesDetach,
            selector: tag(),
            laboratory_id: "lab-on-tag".to_string(),
            base: Default::default(),
        },
        "is not attached",
    )
    .await;
}
