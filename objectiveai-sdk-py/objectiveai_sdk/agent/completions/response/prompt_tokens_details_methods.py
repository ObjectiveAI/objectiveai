"""Methods for PromptTokensDetails."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_option_int
from objectiveai_sdk.agent.completions.response.prompt_tokens_details import (
    PromptTokensDetails,
)


def _push(self, other: PromptTokensDetails) -> None:
    push_option_int(self, "audio_tokens", other.audio_tokens)
    push_option_int(self, "cached_tokens", other.cached_tokens)
    push_option_int(self, "cache_write_tokens", other.cache_write_tokens)
    push_option_int(self, "video_tokens", other.video_tokens)


PromptTokensDetails.push = _push
