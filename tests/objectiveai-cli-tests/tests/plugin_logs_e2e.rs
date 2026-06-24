//! E2E: `plugins run` captures a plugin's stderr into the DB, and
//! `plugins logs list` reads it back.
//!
//! Runs the `stderr-plugin` fixture (coordinate `objectiveai/stderr/0.0.1`),
//! which writes three known lines to stderr and exits 0, then verifies the
//! captured lines (order, coordinate stamping) and the `--after-id` /
//! `--limit` cursor.

mod cli_test_util;

use objectiveai_sdk::cli::command::plugins::logs::list::{
    Path as LogsPath, Request as LogsRequest, ResponseItem as LogsItem,
};
use objectiveai_sdk::cli::command::plugins::run::{
    Path as RunPath, Request as RunRequest, ResponseItem as RunItem,
};

const OWNER: &str = "objectiveai";
const NAME: &str = "stderr";
const VERSION: &str = "0.0.1";

fn logs_request(after_id: Option<i64>, limit: Option<i64>) -> LogsRequest {
    LogsRequest {
        path_type: LogsPath::PluginsLogsList,
        owner: OWNER.to_string(),
        name: NAME.to_string(),
        version: VERSION.to_string(),
        after_id,
        limit,
        base: Default::default(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn plugins_run_captures_stderr_readable_via_logs_list() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // (a) Run the fixture. `plugins run` blocks until the child exits AND
    // the stderr writer task is joined, so all three lines are committed
    // to `objectiveai.plugin_messages` by the time this returns. The
    // fixture writes nothing to stdout, so no run items are expected.
    let _run: Vec<RunItem> = cli_test_util::collect_stream(
        &executor,
        RunRequest {
            path_type: RunPath::PluginsRun,
            owner: OWNER.to_string(),
            name: NAME.to_string(),
            version: VERSION.to_string(),
            args: Vec::new(),
            base: Default::default(),
        },
    )
    .await;

    // (b) Read the captured stderr.
    let lines: Vec<LogsItem> = cli_test_util::collect_stream(&executor, logs_request(None, None)).await;

    let captured: Vec<&str> = lines.iter().map(|l| l.line.as_str()).collect();
    assert_eq!(
        captured,
        vec![
            "stderr-plugin line 1",
            "stderr-plugin line 2",
            "stderr-plugin line 3",
        ],
        "captured stderr lines (ordered by index) must match the fixture's output",
    );
    // `index` is strictly ascending across rows.
    assert!(
        lines.windows(2).all(|w| w[0].index < w[1].index),
        "rows must be returned in ascending index order",
    );
    // The plugin coordinate is stamped on every row.
    assert!(
        lines
            .iter()
            .all(|l| l.owner == OWNER && l.name == NAME && l.version == VERSION),
        "every row must carry the plugin coordinate",
    );

    // (c) Cursor + limit: after the first row, capped at 1, returns only
    // the second line.
    let first = lines[0].index;
    let page: Vec<LogsItem> =
        cli_test_util::collect_stream(&executor, logs_request(Some(first), Some(1))).await;
    assert_eq!(page.len(), 1, "--limit 1 must return exactly one row");
    assert_eq!(page[0].line, "stderr-plugin line 2");
    assert!(page[0].index > first, "--after-id must skip rows with index <= after_id");
}
