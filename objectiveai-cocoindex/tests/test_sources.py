"""Source ADT tests — memo keys + request-field translation. No network."""

from __future__ import annotations

from objectiveai_cocoindex import (
    InlineFunction,
    InlineProfile,
    RemoteFunction,
    RemoteProfile,
)
from objectiveai_sdk.functions.full_inline_function import (
    FullInlineFunction,
    FullInlineFunctionStandard,
)
from objectiveai_sdk.functions.full_inline_function_or_remote_commit_optional import (
    FullInlineFunctionOrRemoteCommitOptionalInline,
    FullInlineFunctionOrRemoteCommitOptionalRemote,
)
from objectiveai_sdk.functions.inline_function import InlineFunction as InlineFunctionBody
from objectiveai_sdk.functions.inline_function import InlineFunctionScalar
from objectiveai_sdk.functions.inline_profile import (
    InlineProfile as InlineProfileBody,
    InlineProfileAuto,
)
from objectiveai_sdk.functions.inline_profile_or_remote_commit_optional import (
    InlineProfileOrRemoteCommitOptionalInline,
    InlineProfileOrRemoteCommitOptionalRemote,
)
from objectiveai_sdk.swarm.inline_swarm_base import InlineSwarmBase


# ---------------------------------------------------------------------------
# RemoteFunction
# ---------------------------------------------------------------------------


def test_remote_function_github_field_shape():
    rf = RemoteFunction.github(owner="acme", repository="repo", commit="deadbeef")
    field = rf.to_function_field()
    assert isinstance(field.root, FullInlineFunctionOrRemoteCommitOptionalRemote)
    inner = field.root.root.root  # RemotePathCommitOptional → variant
    assert inner.remote == "github"
    assert inner.owner == "acme"
    assert inner.repository == "repo"
    assert inner.commit == "deadbeef"


def test_remote_function_filesystem_field_shape():
    rf = RemoteFunction.filesystem(owner="acme", repository="repo")
    field = rf.to_function_field()
    inner = field.root.root.root
    assert inner.remote == "filesystem"
    assert inner.owner == "acme"
    assert inner.commit is None


def test_remote_function_mock_field_shape():
    rf = RemoteFunction.mock(name="my-mock")
    field = rf.to_function_field()
    inner = field.root.root.root
    assert inner.remote == "mock"
    assert inner.name == "my-mock"


def test_remote_function_memo_key_stable_across_construction():
    a = RemoteFunction.github(owner="acme", repository="repo", commit="abc")
    b = RemoteFunction.github(owner="acme", repository="repo", commit="abc")
    assert a.__coco_memo_key__() == b.__coco_memo_key__()


def test_remote_function_memo_key_differs_on_commit():
    a = RemoteFunction.github(owner="acme", repository="repo", commit="abc")
    b = RemoteFunction.github(owner="acme", repository="repo", commit="def")
    assert a.__coco_memo_key__() != b.__coco_memo_key__()


def test_remote_function_memo_key_differs_on_remote_kind():
    a = RemoteFunction.github(owner="acme", repository="repo")
    b = RemoteFunction.filesystem(owner="acme", repository="repo")
    assert a.__coco_memo_key__() != b.__coco_memo_key__()


# ---------------------------------------------------------------------------
# RemoteProfile
# ---------------------------------------------------------------------------


def test_remote_profile_github_field_shape():
    rp = RemoteProfile.github(owner="acme", repository="profiles", commit="abc")
    field = rp.to_profile_field()
    assert isinstance(field.root, InlineProfileOrRemoteCommitOptionalRemote)
    inner = field.root.root.root
    assert inner.remote == "github"
    assert inner.owner == "acme"
    assert inner.commit == "abc"


def test_remote_profile_memo_key_stable_across_construction():
    a = RemoteProfile.github(owner="acme", repository="repo", commit="abc")
    b = RemoteProfile.github(owner="acme", repository="repo", commit="abc")
    assert a.__coco_memo_key__() == b.__coco_memo_key__()


def test_function_and_profile_memo_keys_disambiguate():
    """A RemoteFunction and a RemoteProfile pointing at the same ref should
    fingerprint differently (the wrapper-class tag is part of the key)."""
    fn = RemoteFunction.github(owner="acme", repository="repo", commit="abc")
    pf = RemoteProfile.github(owner="acme", repository="repo", commit="abc")
    assert fn.__coco_memo_key__() != pf.__coco_memo_key__()


# ---------------------------------------------------------------------------
# InlineFunction
# ---------------------------------------------------------------------------


def _make_inline_scalar_body() -> FullInlineFunction:
    body = InlineFunctionBody(
        # `type_` is the python attr; pydantic exposes the alias `type`.
        root=InlineFunctionScalar.model_validate({"type": "scalar.function", "tasks": []})
    )
    return FullInlineFunction(root=FullInlineFunctionStandard(root=body))


def test_inline_function_field_shape():
    inline = InlineFunction(_make_inline_scalar_body())
    field = inline.to_function_field()
    assert isinstance(field.root, FullInlineFunctionOrRemoteCommitOptionalInline)


def test_inline_function_memo_key_stable():
    a = InlineFunction(_make_inline_scalar_body())
    b = InlineFunction(_make_inline_scalar_body())
    assert a.__coco_memo_key__() == b.__coco_memo_key__()


# ---------------------------------------------------------------------------
# InlineProfile
# ---------------------------------------------------------------------------


def _make_inline_auto_profile() -> InlineProfileBody:
    swarm = InlineSwarmBase(agents=[], weights=None)
    return InlineProfileBody(root=InlineProfileAuto(root=swarm))


def test_inline_profile_field_shape():
    inline = InlineProfile(_make_inline_auto_profile())
    field = inline.to_profile_field()
    assert isinstance(field.root, InlineProfileOrRemoteCommitOptionalInline)


def test_inline_profile_memo_key_stable():
    a = InlineProfile(_make_inline_auto_profile())
    b = InlineProfile(_make_inline_auto_profile())
    assert a.__coco_memo_key__() == b.__coco_memo_key__()
