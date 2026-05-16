//! Tauri command `cli_run` — thin wrapper that hands the cli a
//! [`Handle::Stream`](objectiveai_sdk::cli::output::Handle) from
//! [`cli_event_sink`](objectiveai_sdk::viewer::cli_event_sink) so each
//! emitted JSONL line is forwarded to the iframe that invoked the cli
//! as an [`Event::CliCommand`](objectiveai_sdk::viewer::Event) event.
//!
//! The plugin-bridge resolves the originating iframe (via
//! `MessageEvent.source`) and passes its repository name as `origin`,
//! which becomes the `destination` on every emitted event. The plugin
//! never sets `destination` itself.

use objectiveai_sdk::viewer::{cli_event_sink, EventSender};

/// Drive the cli with `args` in-process. The forwarder
/// (`cli_event_sink`) wraps each emitted line as
/// `Event::CliCommand { destination: origin, value }` and pushes it
/// onto the viewer's events bus, where the JS bridge picks it up and
/// forwards to the originating iframe.
///
/// Returns immediately after spawning the cli + forwarder tasks; the
/// iframe sees output asynchronously via the events channel. When the
/// cli's `run()` completes, the `Handle::Stream` sender is dropped,
/// the forwarder loop exits, and any handler waiting on the stream
/// terminates naturally on the `{"type":"end"}` line.
#[tauri::command]
pub async fn cli_run(
    events_tx: tauri::State<'_, EventSender>,
    args: Vec<String>,
    origin: String,
) -> Result<(), String> {
    let handle = cli_event_sink(events_tx.inner().clone(), origin);
    let cli_config = objectiveai_cli::load_config();
    tokio::spawn(async move {
        let _exit_code = objectiveai_cli::run(args, &cli_config, handle).await;
    });
    Ok(())
}
