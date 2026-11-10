//! The enqueue verb.

use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use uuid::Uuid;

use super::echoes;
use super::stdin;
use super::writer;

/// No run is open to write to — none started, or the one there was
/// has ended: refused, `409` `{"kind":"closed"}`, and nothing
/// written.
pub struct Closed;

/// Hand the proxy's batch to the running session: written as ONE
/// user message — every message's blocks, in order — and answered
/// the moment the write lands. Nothing waits for Claude Code to take
/// it: the write IS the delivery, and Claude Code takes the message
/// from its own queue at a seam of its choosing.
///
/// The writer lock is held throughout, so the run's close — which
/// takes it to clear the stdin — is never interleaved: an enqueue
/// either writes into the open run or finds it [`Closed`]. Under the
/// lock, the batch is registered among the [`echoes`] BEFORE the
/// write, so its echo always finds it, and taken back out when the
/// write fails. No run to write to, or a failed write — a broken
/// stdin is the process dying, the run over — is [`Closed`].
pub async fn enqueue(messages: Vec<Message>, blocks: Vec<stdin::Block>) -> Result<(), Closed> {
    let uuid = Uuid::new_v4().to_string();
    let mut writer = writer::WRITER.lock().await;
    let Some(child_stdin) = writer.as_mut() else {
        return Err(Closed);
    };
    echoes::ECHOES.lock().await.push(echoes::Written { uuid: uuid.clone(), messages });
    match stdin::write_lines(child_stdin, &stdin::user_message_line(blocks, uuid.clone())).await {
        Ok(()) => Ok(()),
        Err(_) => {
            *writer = None;
            echoes::ECHOES.lock().await.retain(|written| written.uuid != uuid);
            Err(Closed)
        }
    }
}
