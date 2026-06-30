//! E2E: `laboratories create` + `laboratories list` (real podman).
//!
//! Gated behind `OBJECTIVEAI_TEST_PODMAN=1`: `laboratories create` shells
//! out to podman (which the CLI provisions on demand), so this is opt-in
//! to keep the default suite podman-free. With the gate set, it creates a
//! laboratory container (created, not started) and confirms `list` reads
//! the spec back from the container's `objectiveai.laboratory` label.

mod cli_test_util;

use objectiveai_sdk::cli::command::laboratories::create::{
    EnvVar, Kind, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use objectiveai_sdk::cli::command::laboratories::list::{
    Path as ListPath, Request as ListReq, ResponseItem as ListItem,
};

/// Skip unless the caller opted into podman-backed tests.
fn podman_enabled() -> bool {
    std::env::var("OBJECTIVEAI_TEST_PODMAN").as_deref() == Ok("1")
}

/// A minimal, widely-available base image. `create` only `podman
/// create`s + `podman cp`s into it (no run), so any image works.
const BASE_IMAGE: &str = "docker.io/library/busybox:latest";

#[tokio::test(flavor = "multi_thread")]
async fn create_then_list_round_trips_the_spec() {
    if !podman_enabled() {
        eprintln!("skipping laboratories_e2e: set OBJECTIVEAI_TEST_PODMAN=1 to run");
        return;
    }
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Unique id (state is shared across runs; no `laboratories delete`).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = format!("e2e-lab-{nanos}");

    let created: CreateResp = cli_test_util::execute_one(
        &executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.clone(),
            image: BASE_IMAGE.to_string(),
            mounts: Vec::new(),
            env: vec![EnvVar {
                key: "FOO".to_string(),
                value: "bar".to_string(),
            }],
            cwd: "/work".to_string(),
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
    assert_eq!(created.cwd, "/work");

    let labs: Vec<ListItem> = cli_test_util::collect_stream(
        &executor,
        ListReq {
            path_type: ListPath::LaboratoriesList,
            kind: Kind::Client,
            base: Default::default(),
        },
    )
    .await;

    let found = labs
        .iter()
        .find(|l| l.id == id)
        .unwrap_or_else(|| panic!("created lab {id} not in list: {:?}", labs.iter().map(|l| &l.id).collect::<Vec<_>>()));
    assert_eq!(found.image, BASE_IMAGE);
    assert_eq!(found.cwd, "/work");
    assert!(
        found.env.iter().any(|e| e.key == "FOO" && e.value == "bar"),
        "env not round-tripped: {:?}",
        found.env.iter().map(|e| (&e.key, &e.value)).collect::<Vec<_>>()
    );
}
