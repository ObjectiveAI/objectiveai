"""Module-default ObjectiveAI client.

Used by `Function` instances constructed without an explicit ``client``.
Constructed lazily so importing this module doesn't require auth env vars.
"""

from __future__ import annotations

from objectiveai_sdk.client import ObjectiveAI

_default_instance: ObjectiveAI | None = None


def set_default_client(client: ObjectiveAI | None) -> None:
    """Set (or clear) the process-wide default ``ObjectiveAI`` client.

    ``Function`` instances constructed without an explicit ``client``
    will use this default. If unset, a fresh ``ObjectiveAI()`` is
    constructed lazily on first use (reads ``OBJECTIVEAI_AUTHORIZATION``
    and friends from the environment via ``ObjectiveAI()``'s own
    constructor defaults).
    """
    global _default_instance
    _default_instance = client


def resolve_client() -> ObjectiveAI:
    """Return the current default client, constructing one from env if
    none has been set."""
    global _default_instance
    if _default_instance is None:
        _default_instance = ObjectiveAI()
    return _default_instance
