"""Methods for RichContent."""
from __future__ import annotations

from objectiveai_sdk.agent.completions.message.rich_content import (
    RichContent,
    RichContentText,
    RichContentParts,
)
from objectiveai_sdk.agent.completions.message.rich_content_part import (
    RichContentPart,
    RichContentPartText,
)


def _push(self, other: RichContent) -> None:
    self_inner = self.root
    other_inner = other.root

    self_is_text = isinstance(self_inner, RichContentText)
    other_is_text = isinstance(other_inner, RichContentText)

    if self_is_text and other_is_text:
        # text + text → concatenate
        self_inner.root += other_inner.root
    elif self_is_text and not other_is_text:
        # text + parts → convert self to parts, extend
        text_part = RichContentPartText(
            text=self_inner.root, type="text",
        )
        parts = [RichContentPart(root=text_part)]
        parts.extend(other_inner.root)
        self.root = RichContentParts(root=parts)
    elif not self_is_text and other_is_text:
        # parts + text → append text as new part
        if other_inner.root:
            text_part = RichContentPartText(
                text=other_inner.root, type="text",
            )
            self_inner.root.append(RichContentPart(root=text_part))
    else:
        # parts + parts → extend
        if other_inner.root:
            self_inner.root.extend(other_inner.root)


RichContent.push = _push
