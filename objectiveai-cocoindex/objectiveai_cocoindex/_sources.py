"""ADTs for ObjectiveAI function and profile sources.

A ``Function`` (the executor) takes one ``FunctionSource`` and one
``ProfileSource`` at construction. Each source knows how to:

  - translate itself into the request-field shape expected by
    ``FunctionExecutionCreateParams``;
  - return a stable ``__coco_memo_key__`` so cocoindex's memoization
    fingerprint changes whenever the underlying ref/body changes.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from objectiveai_sdk.functions.full_inline_function import FullInlineFunction
from objectiveai_sdk.functions.full_inline_function_or_remote_commit_optional import (
    FullInlineFunctionOrRemoteCommitOptional,
    FullInlineFunctionOrRemoteCommitOptionalInline,
    FullInlineFunctionOrRemoteCommitOptionalRemote,
)
from objectiveai_sdk.functions.inline_profile import InlineProfile as InlineProfileBody
from objectiveai_sdk.functions.inline_profile_or_remote_commit_optional import (
    InlineProfileOrRemoteCommitOptional,
    InlineProfileOrRemoteCommitOptionalInline,
    InlineProfileOrRemoteCommitOptionalRemote,
)
from objectiveai_sdk.remote_path_commit_optional import (
    RemotePathCommitOptional,
    RemotePathCommitOptionalFilesystem,
    RemotePathCommitOptionalGithub,
    RemotePathCommitOptionalMock,
)


@runtime_checkable
class FunctionSource(Protocol):
    """Either an inline function body or a remote function reference."""

    def to_function_field(self) -> FullInlineFunctionOrRemoteCommitOptional: ...
    def __coco_memo_key__(self) -> object: ...


@runtime_checkable
class ProfileSource(Protocol):
    """Either an inline profile body or a remote profile reference."""

    def to_profile_field(self) -> InlineProfileOrRemoteCommitOptional: ...
    def __coco_memo_key__(self) -> object: ...


# ---------------------------------------------------------------------------
# Remote sources
# ---------------------------------------------------------------------------


class RemoteFunction:
    """Reference to a remotely-hosted function (GitHub, filesystem, mock).

    Use the ``github`` / ``filesystem`` / ``mock`` classmethods rather than
    the bare constructor.
    """

    __slots__ = ("_ref",)

    def __init__(self, ref: RemotePathCommitOptional) -> None:
        self._ref = ref

    @classmethod
    def github(
        cls,
        *,
        owner: str,
        repository: str,
        commit: str | None = None,
    ) -> RemoteFunction:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalGithub(
                    remote="github", owner=owner, repository=repository, commit=commit,
                )
            )
        )

    @classmethod
    def filesystem(
        cls,
        *,
        owner: str,
        repository: str,
        commit: str | None = None,
    ) -> RemoteFunction:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalFilesystem(
                    remote="filesystem", owner=owner, repository=repository, commit=commit,
                )
            )
        )

    @classmethod
    def mock(cls, *, name: str) -> RemoteFunction:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalMock(remote="mock", name=name)
            )
        )

    @property
    def ref(self) -> RemotePathCommitOptional:
        return self._ref

    def to_function_field(self) -> FullInlineFunctionOrRemoteCommitOptional:
        return FullInlineFunctionOrRemoteCommitOptional(
            root=FullInlineFunctionOrRemoteCommitOptionalRemote(root=self._ref)
        )

    def __coco_memo_key__(self) -> object:
        return ("objectiveai_cocoindex.RemoteFunction", self._ref.model_dump())

    def __repr__(self) -> str:
        return f"RemoteFunction({self._ref.root!r})"


class RemoteProfile:
    """Reference to a remotely-hosted profile (GitHub, filesystem, mock)."""

    __slots__ = ("_ref",)

    def __init__(self, ref: RemotePathCommitOptional) -> None:
        self._ref = ref

    @classmethod
    def github(
        cls,
        *,
        owner: str,
        repository: str,
        commit: str | None = None,
    ) -> RemoteProfile:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalGithub(
                    remote="github", owner=owner, repository=repository, commit=commit,
                )
            )
        )

    @classmethod
    def filesystem(
        cls,
        *,
        owner: str,
        repository: str,
        commit: str | None = None,
    ) -> RemoteProfile:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalFilesystem(
                    remote="filesystem", owner=owner, repository=repository, commit=commit,
                )
            )
        )

    @classmethod
    def mock(cls, *, name: str) -> RemoteProfile:
        return cls(
            RemotePathCommitOptional(
                root=RemotePathCommitOptionalMock(remote="mock", name=name)
            )
        )

    @property
    def ref(self) -> RemotePathCommitOptional:
        return self._ref

    def to_profile_field(self) -> InlineProfileOrRemoteCommitOptional:
        return InlineProfileOrRemoteCommitOptional(
            root=InlineProfileOrRemoteCommitOptionalRemote(root=self._ref)
        )

    def __coco_memo_key__(self) -> object:
        return ("objectiveai_cocoindex.RemoteProfile", self._ref.model_dump())

    def __repr__(self) -> str:
        return f"RemoteProfile({self._ref.root!r})"


# ---------------------------------------------------------------------------
# Inline sources
# ---------------------------------------------------------------------------


class InlineFunction:
    """Inline function definition. Accepts a ``FullInlineFunction`` body
    constructed from ``objectiveai.functions``.
    """

    __slots__ = ("_body",)

    def __init__(self, body: FullInlineFunction) -> None:
        self._body = body

    @property
    def body(self) -> FullInlineFunction:
        return self._body

    def to_function_field(self) -> FullInlineFunctionOrRemoteCommitOptional:
        return FullInlineFunctionOrRemoteCommitOptional(
            root=FullInlineFunctionOrRemoteCommitOptionalInline(root=self._body)
        )

    def __coco_memo_key__(self) -> object:
        return ("objectiveai_cocoindex.InlineFunction", self._body.model_dump())

    def __repr__(self) -> str:
        return f"InlineFunction({self._body!r})"


class InlineProfile:
    """Inline profile definition. Accepts an ``InlineProfile`` body
    constructed from ``objectiveai.functions``.
    """

    __slots__ = ("_body",)

    def __init__(self, body: InlineProfileBody) -> None:
        self._body = body

    @property
    def body(self) -> InlineProfileBody:
        return self._body

    def to_profile_field(self) -> InlineProfileOrRemoteCommitOptional:
        return InlineProfileOrRemoteCommitOptional(
            root=InlineProfileOrRemoteCommitOptionalInline(root=self._body)
        )

    def __coco_memo_key__(self) -> object:
        return ("objectiveai_cocoindex.InlineProfile", self._body.model_dump())

    def __repr__(self) -> str:
        return f"InlineProfile({self._body!r})"
