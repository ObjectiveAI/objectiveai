use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Choice {
    pub delta: super::Delta,
    pub finish_reason: Option<response::FinishReason>,
    pub index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<response::Logprobs>,
}

impl Choice {
    pub fn push(
        &mut self,
        Choice {
            delta,
            finish_reason,
            logprobs,
            ..
        }: &Choice,
    ) {
        self.delta.push(delta);
        if self.finish_reason.is_none() {
            self.finish_reason = finish_reason.clone();
        }
        match (&mut self.logprobs, logprobs) {
            (Some(self_logprobs), Some(other_logprobs)) => {
                self_logprobs.push(other_logprobs);
            }
            (None, Some(other_logprobs)) => {
                self.logprobs = Some(other_logprobs.clone());
            }
            _ => {}
        }
    }
}
