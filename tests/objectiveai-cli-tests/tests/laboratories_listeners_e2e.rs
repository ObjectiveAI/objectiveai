//! E2E: the daemon's LIVE laboratories WebSocket endpoints, consumed
//! with the Rust SDK listener clients (their first Rust consumers):
//!
//! - `/laboratories/list` ([`WebSocketLaboratoriesListListener`]) —
//!   the host-registry stream (Snapshot / Upserted / Removed).
//! - `/laboratories/{id}` ([`WebSocketLaboratoriesListener`]) — one
//!   laboratory's full record (identity + machine + attachments),
//!   full-value replaced on every relevant change.
//!
//! Everything is driven through CLI commands — no direct filesystem
//! writes; each daemon's `http://` address comes from its published
//! lockfile (read-only, the CLI's own discovery mechanism), and the
//! host's dial list rides `laboratories config addresses add`.
//!
//! The cross-daemon test runs TWO daemons (different states, one
//! machine): state A's host dials both, so a create/delete issued via
//! EITHER daemon must appear on BOTH daemons' streams — the host's
//! HostIdentify announce + HostNotification fan-out is the only
//! propagation mechanism (no scans, no polling).

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::SetScope;
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::laboratories::config::addresses::add::{
    Path as AddrAddPath, Request as AddrAddReq, Response as AddrAddResp,
};
use objectiveai_sdk::cli::command::laboratories::create::{
    Kind, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::laboratories::delete::{
    Kind as DeleteKind, Path as DeletePath, Request as DeleteReq, Response as DeleteResp,
};
use objectiveai_sdk::cli::command::laboratories::kill::{
    Path as LabKillPath, Request as LabKillReq, Response as LabKillResp,
};
use objectiveai_sdk::cli::command::laboratories::list::{
    Path as ListPath, Request as ListReq, ResponseItem as ListItem,
};
use objectiveai_sdk::cli::command::laboratories::spawn::{
    Path as LabSpawnPath, Request as LabSpawnReq, Response as LabSpawnResp,
};
use objectiveai_sdk::cli::websocket_laboratories_list_listener::WebSocketLaboratoriesListListener;
use objectiveai_sdk::cli::websocket_laboratories_listener::{
    LaboratoryAttachment, WebSocketLaboratoriesListener,
};

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

/// A minimal, widely-available base image for the laboratory.
/// The split base image every lab in this file uses —
/// `docker.io/library/busybox:latest`, as parts (a joined reference string
/// is unrepresentable in the API).
fn base_image() -> objectiveai_sdk::laboratories::LaboratoryImage {
    objectiveai_sdk::laboratories::LaboratoryImage::Registry(
        objectiveai_sdk::laboratories::RegistryLaboratoryImage {
        registry: "docker.io".to_string(),
        name: "library/busybox".to_string(),
            pin: objectiveai_sdk::laboratories::LaboratoryImagePin::Tag(
                "latest".to_string(),
            ),
        },
    )
}

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

/// Create a laboratory on the given host — `pair` = the explicit
/// `--machine`/`--machine-state` target, `None` = the executor
/// daemon's own (local machine, own state).
async fn create_lab_on(executor: &Exec, id: &str, pair: Option<(&str, &str)>) {
    let created: CreateResp = cli_test_util::execute_one(
        executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.to_string(),
            image: base_image(),
            mounts: Vec::new(),
            env: Vec::new(),
            cwd: "/".to_string(),
            machine: pair.map(|(machine, _)| machine.to_string()),
            machine_state: pair.map(|(_, machine_state)| machine_state.to_string()),
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
}

async fn create_lab(executor: &Exec, id: &str) {
    create_lab_on(executor, id, None).await;
}

async fn delete_lab_on(executor: &Exec, id: &str, pair: Option<(&str, &str)>) {
    let deleted: DeleteResp = cli_test_util::execute_one(
        executor,
        DeleteReq {
            path_type: DeletePath::LaboratoriesDelete,
            kind: DeleteKind::Client,
            id: id.to_string(),
            machine: pair.map(|(machine, _)| machine.to_string()),
            machine_state: pair.map(|(_, machine_state)| machine_state.to_string()),
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(deleted.id, id);
}

async fn delete_lab(executor: &Exec, id: &str) {
    delete_lab_on(executor, id, None).await;
}

/// This machine's stable hashed id — what every listed laboratory's
/// `machine.id` must equal (the daemon computes the same value
/// independently; that is the machine-identity design).
fn local_machine_id() -> String {
    objectiveai_sdk::machine::machine_id(&cli_test_util::objectiveai_dir())
}

/// Single daemon: the `/laboratories/list` stream and the
/// `/laboratories/{id}` record stream across the laboratory's whole
/// life — attachment-only (zero-filled identity), created (identity +
/// machine + connected), deleted (zero-filled again, attachment rows
/// surviving).
#[tokio::test(flavor = "multi_thread")]
async fn laboratories_list_and_record_streams() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;

    let id = format!("e2e-listeners-lab-{}", nanos());

    // Attach FIRST (pure DB rows — no podman, no host): the record
    // stream must serve attachment-only laboratories with zero-filled
    // identity, and doing this first also brings the DB up so every
    // later record rebuild (driven by registry events) sees the row.
    let spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        serde_json::json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "hi"
        }),
    )
    .expect("mock agent spec deserializes");
    let tag = format!("listeners-lab-tag-{}", nanos());
    let _: ApplyResp = cli_test_util::execute_one(
        &executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.clone(),
            target: ApplyTarget::Agent {
                agent_spec: spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    let _: AttachResp = cli_test_util::execute_one(
        &executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Tag { agent_tag: tag.clone() },
            laboratory_id: id.clone(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;

    // Listeners up-front: the list snapshot must NOT contain the
    // attachment-only laboratory (nothing serves it); the record
    // stream must serve it zero-filled with the attachment row.
    let list = WebSocketLaboratoriesListListener::new(format!("{addr}/laboratories/list"))
        .connect()
        .await
        .expect("connect /laboratories/list");
    let record = WebSocketLaboratoriesListener::new(format!("{addr}/laboratories/{id}"))
        .connect()
        .await
        .expect("connect /laboratories/{id}");

    wait_for!("attachment-only record", {
        match record.laboratory().await {
            Some(rec) => {
                assert_eq!(rec.id, id);
                assert!(!rec.connected, "nothing serves the lab yet");
                assert!(rec.machine.is_none(), "no serving host yet");
                matches!(
                    rec.attachments.as_slice(),
                    [LaboratoryAttachment::Tag { tag: t, .. }] if *t == tag
                )
            }
            None => false,
        }
    });
    assert!(
        !list.laboratories().await.iter().any(|l| l.id == id),
        "attachment-only laboratories must not ride the list stream"
    );

    // Create → the list stream upserts it (served, connected, THIS
    // machine) and the record fills its identity while keeping the
    // attachment.
    create_lab(&executor, &id).await;
    let machine_id = local_machine_id();
    wait_for!("created lab on the list stream", {
        list.laboratories().await.iter().any(|l| {
            l.id == id
                && l.connected
                && l.image == base_image()
                && l.machine.as_ref().map(|m| m.id.as_str()) == Some(machine_id.as_str())
        })
    });
    wait_for!("created lab record", {
        match record.laboratory().await {
            Some(rec) => {
                rec.connected
                    && rec.image.as_ref() == Some(&base_image())
                    && rec.machine.as_ref().map(|m| m.id.as_str())
                        == Some(machine_id.as_str())
                    && rec.attachments.len() == 1
            }
            None => false,
        }
    });

    // Delete → the list stream removes it; the record zero-fills its
    // identity again but KEEPS the attachment row (rows outlive the
    // laboratory).
    delete_lab(&executor, &id).await;
    wait_for!("deleted lab gone from the list stream", {
        !list.laboratories().await.iter().any(|l| l.id == id)
    });
    wait_for!("deleted lab record zero-fills", {
        match record.laboratory().await {
            Some(rec) => {
                !rec.connected
                    && rec.machine.is_none()
                    && rec.image.is_none()
                    && rec.attachments.len() == 1
            }
            None => false,
        }
    });
}

/// TWO daemons, different states, one machine: state A's host dials
/// both (the state-B address configured via `laboratories config
/// addresses add`), and creates/deletes issued via EITHER daemon
/// propagate to BOTH daemons' `/laboratories/list` streams through
/// the host's announce + notification fan-out.
#[tokio::test(flavor = "multi_thread")]
async fn laboratories_cross_daemon_propagation() {
    let _base = cli_test_util::test_base_dir();
    let state_a = cli_test_util::test_state_name();
    let state_b = format!("{state_a}-b");
    let exec_a = cli_test_util::executor().await;
    let exec_b = cli_test_util::executor_for_state(&state_b).await;

    let addr_a = cli_test_util::daemon_address(&exec_a, &state_a).await;
    let addr_b = cli_test_util::daemon_address(&exec_b, &state_b).await;
    assert_ne!(addr_a, addr_b, "two daemons must bind distinct ports");

    // Point state A's host at daemon B too (empty value ⇒ dial
    // unauthenticated — test daemons run secretless), then spawn it:
    // ONE host process, TWO daemon connections.
    let _: AddrAddResp = cli_test_util::execute_one(
        &exec_a,
        AddrAddReq {
            path_type: AddrAddPath::LaboratoriesConfigAddressesAdd,
            scope: SetScope::State,
            key: addr_b.clone(),
            value: String::new(),
            base: Default::default(),
        },
    )
    .await;
    let spawned: LabSpawnResp = cli_test_util::execute_one(
        &exec_a,
        LabSpawnReq {
            path_type: LabSpawnPath::LaboratoriesSpawn,
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(
        spawned.addresses,
        vec![addr_a.clone(), addr_b.clone()],
        "the host dials the local daemon first, then the configured address"
    );

    let list_a = WebSocketLaboratoriesListListener::new(format!("{addr_a}/laboratories/list"))
        .connect()
        .await
        .expect("connect daemon A /laboratories/list");
    let list_b = WebSocketLaboratoriesListListener::new(format!("{addr_b}/laboratories/list"))
        .connect()
        .await
        .expect("connect daemon B /laboratories/list");

    // Create via daemon A → BOTH daemons see it (host notification
    // fan-out; daemon B has no host of its own).
    let lab1 = format!("e2e-xdaemon-lab1-{}", nanos());
    create_lab(&exec_a, &lab1).await;
    let machine_id = local_machine_id();
    for (name, list) in [("A", &list_a), ("B", &list_b)] {
        wait_for!(format!("lab1 on daemon {name}'s list stream"), {
            list.laboratories().await.iter().any(|l| {
                l.id == lab1
                    && l.connected
                    && l.machine.as_ref().map(|m| m.id.as_str())
                        == Some(machine_id.as_str())
            })
        });
    }

    // Create via daemon B ("from a different remote") with the
    // EXPLICIT --machine/--machine-state pair addressing state A's
    // host (the naive default would be daemon B's own (machine,
    // state B) and auto-spawn a second host). BOTH daemons must see
    // the new laboratory — the create rides daemon B's connection to
    // the shared host, the update fans out to daemon A.
    let lab2 = format!("e2e-xdaemon-lab2-{}", nanos());
    create_lab_on(&exec_b, &lab2, Some((machine_id.as_str(), state_a.as_str()))).await;
    for (name, list) in [("A", &list_a), ("B", &list_b)] {
        wait_for!(format!("lab2 on daemon {name}'s list stream"), {
            list.laboratories().await.iter().any(|l| l.id == lab2 && l.connected)
        });
    }
    // The unary list through daemon A agrees (its registry got the
    // notification even though the create rode daemon B).
    let labs_a: Vec<ListItem> = cli_test_util::collect_stream(
        &exec_a,
        ListReq {
            path_type: ListPath::LaboratoriesList,
            kind: Kind::Client,
            base: Default::default(),
        },
    )
    .await;
    assert!(
        labs_a.iter().any(|l| l.id == lab2),
        "daemon A's unary list must include the lab created via daemon B"
    );

    // Delete via daemon B (the explicit pair addresses state A's
    // host) → gone from BOTH streams; then lab1 via daemon A (its own
    // default pair) likewise.
    delete_lab_on(&exec_b, &lab2, Some((machine_id.as_str(), state_a.as_str()))).await;
    for (name, list) in [("A", &list_a), ("B", &list_b)] {
        wait_for!(format!("lab2 removed from daemon {name}'s list stream"), {
            !list.laboratories().await.iter().any(|l| l.id == lab2)
        });
    }
    delete_lab(&exec_a, &lab1).await;
    for (name, list) in [("A", &list_a), ("B", &list_b)] {
        wait_for!(format!("lab1 removed from daemon {name}'s list stream"), {
            !list.laboratories().await.iter().any(|l| l.id == lab1)
        });
    }

    // Teardown: kill the host (state A's `laboratories` lock) and
    // daemon B — best-effort, so a failed assert above still leaves
    // the usual per-state cleanup to the suite scripts.
    let _: LabKillResp = cli_test_util::execute_one(
        &exec_a,
        LabKillReq {
            path_type: LabKillPath::LaboratoriesKill,
            scope: SetScope::State,
            base: Default::default(),
        },
    )
    .await;
    let _ = exec_b
        .execute_one::<objectiveai_sdk::cli::command::daemon::kill::Request, objectiveai_sdk::cli::command::daemon::kill::Response>(
            objectiveai_sdk::cli::command::daemon::kill::Request {
                path_type: objectiveai_sdk::cli::command::daemon::kill::Path::DaemonKill,
                base: Default::default(),
            },
            None,
        )
        .await;
}

/// The SAME laboratory id on TWO hosts (one machine, two states —
/// each daemon's default pair targets its own host): the second
/// create must NOT error (ids are only unique per (machine, state);
/// there is no cross-host duplicate check), each daemon's list stream
/// shows ITS copy with its own `machine_state`, and a default-pair
/// delete removes ONLY that host's copy.
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_ids_across_hosts() {
    let _base = cli_test_util::test_base_dir();
    let state_a = cli_test_util::test_state_name();
    let state_b = format!("{state_a}-b");
    let exec_a = cli_test_util::executor().await;
    let exec_b = cli_test_util::executor_for_state(&state_b).await;

    let addr_a = cli_test_util::daemon_address(&exec_a, &state_a).await;
    let addr_b = cli_test_util::daemon_address(&exec_b, &state_b).await;

    let list_a = WebSocketLaboratoriesListListener::new(format!("{addr_a}/laboratories/list"))
        .connect()
        .await
        .expect("connect daemon A /laboratories/list");
    let list_b = WebSocketLaboratoriesListListener::new(format!("{addr_b}/laboratories/list"))
        .connect()
        .await
        .expect("connect daemon B /laboratories/list");

    // One id, two hosts: each create's default pair targets that
    // daemon's OWN (machine, state) host — the hosts are independent
    // (state A's dials only daemon A, state B's only daemon B), so
    // the second create lands on a DIFFERENT laboratory daemon and
    // must not collide.
    let dup = format!("e2e-dup-id-{}", nanos());
    create_lab(&exec_a, &dup).await;
    create_lab(&exec_b, &dup).await;

    wait_for!("dup id on daemon A with state A", {
        list_a.laboratories().await.iter().any(|l| {
            l.id == dup && l.machine_state.as_deref() == Some(state_a.as_str())
        })
    });
    wait_for!("dup id on daemon B with state B", {
        list_b.laboratories().await.iter().any(|l| {
            l.id == dup && l.machine_state.as_deref() == Some(state_b.as_str())
        })
    });

    // Default-pair delete via daemon B removes ONLY its host's copy;
    // daemon A's copy survives.
    delete_lab(&exec_b, &dup).await;
    wait_for!("dup id gone from daemon B", {
        !list_b.laboratories().await.iter().any(|l| l.id == dup)
    });
    assert!(
        list_a
            .laboratories()
            .await
            .iter()
            .any(|l| l.id == dup
                && l.machine_state.as_deref() == Some(state_a.as_str())),
        "daemon A's same-id laboratory must survive daemon B's delete"
    );
    delete_lab(&exec_a, &dup).await;
    wait_for!("dup id gone from daemon A", {
        !list_a.laboratories().await.iter().any(|l| l.id == dup)
    });

    // Teardown: both hosts + daemon B (state A's daemon stays, like
    // every other test).
    for exec in [&exec_a, &exec_b] {
        let _: LabKillResp = cli_test_util::execute_one(
            exec,
            LabKillReq {
                path_type: LabKillPath::LaboratoriesKill,
                scope: SetScope::State,
                base: Default::default(),
            },
        )
        .await;
    }
    let _ = exec_b
        .execute_one::<objectiveai_sdk::cli::command::daemon::kill::Request, objectiveai_sdk::cli::command::daemon::kill::Response>(
            objectiveai_sdk::cli::command::daemon::kill::Request {
                path_type: objectiveai_sdk::cli::command::daemon::kill::Path::DaemonKill,
                base: Default::default(),
            },
            None,
        )
        .await;
}
