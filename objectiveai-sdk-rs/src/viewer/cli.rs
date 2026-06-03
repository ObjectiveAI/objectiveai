//! Bridge between [`crate::cli::output::Handle::Stream`] and the viewer
//! [`Event`](super::Event) bus.
//!
//! The cli emits [`Output<T>`](crate::cli::output::Output) envelopes
//! into a `Handle::Stream` mpsc sink. The viewer wants each line
//! repackaged as an [`Event::CliCommand`](super::Event::CliCommand) and
//! pushed onto its events channel for fan-out to the originating
//! iframe. [`cli_event_sink`] wires those two ends together so the
//! viewer's Tauri command stays a thin wrapper around an
//! [`objectiveai_cli::run`](https://docs.rs/objectiveai-cli) call.

use tokio::sync::mpsc;

use super::{Event, EventSender};
use crate::cli::output::{Handle, HandleDestination};

/// Returns a [`Handle::Stream`] that forwards every emitted cli
/// `Output<Value>` line onto `events_tx` as
/// [`Event::CliCommand { destination, value }`](super::Event::CliCommand).
///
/// Spawns a `tokio::task` per call to drive the forwarder loop. The
/// task lives until the returned `Handle` is dropped (which closes the
/// sender side of the internal channel and ends the loop). Failed
/// `events_tx.send` calls (receiver dropped) silently break the loop —
/// same semantics as `Handle::Stdout` writing to a closed pipe.
///
/// `value` is the cli's `Output` envelope as a `serde_json::Value`
/// (`{"type": "error"|"notification", ...}`). JSON encoding cannot
/// fail for `Output`; the `unwrap_or(Value::Null)` is defensive only.
pub fn cli_event_sink(events_tx: EventSender, destination: String) -> Handle {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handle = Handle::from(HandleDestination::Stream(tx));
    tokio::spawn(async move {
        while let Some(output) = rx.recv().await {
            let value = serde_json::to_value(&output)
                .unwrap_or(serde_json::Value::Null);
            if events_tx
                .send(Event::CliCommand {
                    destination: destination.clone(),
                    value,
                })
                .is_err()
            {
                break;
            }
        }
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::output::{
        Notification, NotificationValue, OK, Ok, Output, TypedNotificationValue,
    };

    #[tokio::test]
    async fn forwards_each_output_as_cli_command_event() {
        let (events_tx, mut events_rx) = mpsc::unbounded_channel();
        let handle = cli_event_sink(events_tx, "my_plugin".to_string());

        Output::Notification(Notification {
            value: NotificationValue::Typed(TypedNotificationValue::Ok(OK)),
        })
        .emit(&handle)
        .await;
        Output::Notification(Notification {
            value: NotificationValue::Typed(TypedNotificationValue::Ok(Ok {
                ok: false,
            })),
        })
        .emit(&handle)
        .await;

        // Drop the handle so the forwarder loop exits cleanly.
        drop(handle);

        let mut received = Vec::new();
        while let Some(event) = events_rx.recv().await {
            received.push(event);
        }

        assert_eq!(received.len(), 2);
        for event in &received {
            match event {
                Event::CliCommand { destination, .. } => {
                    assert_eq!(destination, "my_plugin");
                }
                other => panic!("expected CliCommand, got {other:?}"),
            }
        }

        let Event::CliCommand { value: v0, .. } = &received[0] else {
            unreachable!()
        };
        assert_eq!(v0["type"], "ok");
        assert_eq!(v0["ok"], true);

        let Event::CliCommand { value: v1, .. } = &received[1] else {
            unreachable!()
        };
        assert_eq!(v1["type"], "ok");
        assert_eq!(v1["ok"], false);
    }

    #[tokio::test]
    async fn dropping_events_receiver_stops_forwarder() {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let handle = cli_event_sink(events_tx, "p".to_string());

        // Receiver gone — next send from the forwarder will fail and break the loop.
        drop(events_rx);

        // Push one output to trigger a send attempt; the forwarder breaks out.
        Output::Notification(Notification {
            value: NotificationValue::Typed(TypedNotificationValue::Ok(OK)),
        })
        .emit(&handle)
        .await;
        // Dropping the handle would also close the loop, but we want to
        // exercise the send-error path. Give the spawned task a tick.
        tokio::task::yield_now().await;

        drop(handle);
    }
}
