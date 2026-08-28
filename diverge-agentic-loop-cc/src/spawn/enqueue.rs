//! The enqueue verb.

use diverge_provider_sdk::agentic_loop_container;
use uuid::Uuid;

use super::session;
use super::stdin;

/// Queue a message for the running session.
///
/// The write landing is the good answer: Claude Code holds the queue
/// from here, and short of a dequeue withdrawing it the message will
/// enter the conversation. No run to write to — never started, or
/// already over — is the expected failure, and the message is
/// missed.
pub async fn enqueue(
    prompt: String,
) -> agentic_loop_container::enqueue::Response {
    let mut session = session::SESSION.lock().await;
    let Some(inner) = session.as_mut() else {
        return agentic_loop_container::enqueue::Response::Missed {
            r#type: Default::default(),
        };
    };
    let uuid = Uuid::new_v4().to_string();
    match stdin::write_lines(
        &mut inner.stdin,
        &stdin::user_message_line(prompt, uuid.clone()),
    )
    .await
    {
        Ok(()) => {
            inner.queued.push(uuid);
            agentic_loop_container::enqueue::Response::Delivered {
                r#type: Default::default(),
            }
        }
        // A broken stdin is the process dying: the session is over.
        Err(_) => {
            *session = None;
            agentic_loop_container::enqueue::Response::Missed {
                r#type: Default::default(),
            }
        }
    }
}
