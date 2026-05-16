"""Methods for CompletionTokensDetails."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_option_int
from objectiveai_sdk.agent.completions.response.completion_tokens_details import (
    CompletionTokensDetails,
)


def _push(self, other: CompletionTokensDetails) -> None:
    push_option_int(self, "accepted_prediction_tokens", other.accepted_prediction_tokens)
    push_option_int(self, "audio_tokens", other.audio_tokens)
    push_option_int(self, "reasoning_tokens", other.reasoning_tokens)
    push_option_int(self, "rejected_prediction_tokens", other.rejected_prediction_tokens)


CompletionTokensDetails.push = _push
