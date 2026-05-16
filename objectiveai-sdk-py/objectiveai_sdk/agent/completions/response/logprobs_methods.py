"""Methods for Logprobs."""
from __future__ import annotations

from objectiveai_sdk.agent.completions.response.logprobs import (
    Logprobs,
)


def _push(self, other: Logprobs) -> None:
    if self.content is not None and other.content is not None:
        self.content.extend(other.content)
    elif other.content is not None:
        self.content = list(other.content)

    if self.refusal is not None and other.refusal is not None:
        self.refusal.extend(other.refusal)
    elif other.refusal is not None:
        self.refusal = list(other.refusal)


Logprobs.push = _push
