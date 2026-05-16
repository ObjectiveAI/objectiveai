"""
Strict roundtrip test harness for Pydantic JSON Schema validation.

THIS FILE MUST NEVER BE MODIFIED.

This harness is purposefully strict. It loads the original JSON schemas from
objectiveai-json-schema/ exactly as they are on disk — no normalization, no
massaging, no xfail. The original schema is treated as the canonical source
of truth and is never altered.

The contract is simple: the caller passes a schema title and a dict. This
harness loads the original, serializes both sides using the canonical key
ordering from the JSON schema builder (objectiveai-json-schema/builder/),
and compares the serialized strings for exact equality.

Key ordering rules (matching the Rust builder):
  - Inside "properties": keys are sorted alphabetically.
  - Outside "properties": keys are sorted by KEYWORD_ORDER, with any
    unknown keys placed at the end.

If a test fails, the fix belongs in the caller's conversion/normalization
logic or in the Pydantic code generator — never in this file.
"""

import json
from pathlib import Path
from typing import Any

SCHEMA_DIR = Path(__file__).resolve().parent.parent.parent / "objectiveai-json-schema"

# Canonical key ordering for JSON Schema keywords.
# Matches KEYWORD_ORDER in objectiveai-json-schema/builder/src/main.rs.
KEYWORD_ORDER: list[str] = [
    "title",
    "description",
    "type",
    "enum",
    "anyOf",
    "$ref",
    "properties",
    "additionalProperties",
    "items",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "pattern",
    "format",
    "default",
    "omitempty",
]

_KEYWORD_RANK: dict[str, int] = {kw: i for i, kw in enumerate(KEYWORD_ORDER)}
_UNKNOWN_RANK: int = len(KEYWORD_ORDER)


def _order_keys(value: Any, inside_properties: bool) -> Any:
    """Recursively reorder keys to match the Rust builder's canonical order.

    - Inside ``properties``: keys (field names) are sorted alphabetically.
    - Outside ``properties``: keys are sorted by ``KEYWORD_ORDER``, with
      unknown keys placed at the end (preserving their relative order).
    """
    if isinstance(value, dict):
        # Recurse first
        recursed = {
            k: _order_keys(v, inside_properties=k == "properties")
            for k, v in value.items()
        }
        # Sort this level's keys
        if inside_properties:
            sorted_items = sorted(recursed.items(), key=lambda kv: kv[0])
        else:
            sorted_items = sorted(
                recursed.items(),
                key=lambda kv: _KEYWORD_RANK.get(kv[0], _UNKNOWN_RANK),
            )
        return dict(sorted_items)

    if isinstance(value, list):
        return [_order_keys(v, inside_properties=False) for v in value]

    return value


def _serialize(schema: dict) -> str:
    """Serialize a schema dict to a canonical JSON string.

    Applies the builder's key ordering, then pretty-prints with 2-space
    indent (matching ``serde_json::to_string_pretty``).
    """
    ordered = _order_keys(schema, inside_properties=False)
    return json.dumps(ordered, indent=2)


def load_original_json_schemas() -> dict[str, dict]:
    """Load all JSON schemas from objectiveai-json-schema/ exactly as-is.

    Returns a dict mapping each schema's ``title`` to its raw parsed content.
    No normalization is applied.
    """
    schemas: dict[str, dict] = {}
    for f in sorted(SCHEMA_DIR.glob("*.json")):
        content = json.loads(f.read_text(encoding="utf-8"))
        if "title" in content:
            schemas[content["title"]] = content
    return schemas


# ---------------------------------------------------------------------------
# Preloaded schemas — available to importers
# ---------------------------------------------------------------------------

ORIGINAL_SCHEMAS: dict[str, dict] = load_original_json_schemas()
ALL_TITLES: set[str] = set(ORIGINAL_SCHEMAS.keys())


def assert_schema_matches(title: str, converted: dict) -> None:
    """Assert that a converted schema exactly matches the original on disk.

    Both the original and ``converted`` are serialized using the canonical
    key ordering before comparison, so key order differences alone will not
    cause spurious failures — but every key, value, and nesting level must
    match exactly.

    Parameters
    ----------
    title:
        The schema title.  Must exist in ``ORIGINAL_SCHEMAS``.
    converted:
        The caller's Pydantic-derived JSON Schema dict, already normalized
        however the caller sees fit.

    Raises
    ------
    AssertionError
        If the serialized forms differ.
    KeyError
        If ``title`` is not found in the original schemas.
    """
    original = ORIGINAL_SCHEMAS[title]
    expected_str = _serialize(original)
    actual_str = _serialize(converted)
    assert actual_str == expected_str, (
        f"Schema mismatch for '{title}':\n"
        f"\n--- Expected (original from objectiveai-json-schema/) ---\n"
        f"{expected_str}\n"
        f"\n--- Got (Pydantic-derived) ---\n"
        f"{actual_str}"
    )
