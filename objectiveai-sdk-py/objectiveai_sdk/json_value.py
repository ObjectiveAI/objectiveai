"""JsonValue — Python equivalent of json.ts in the JS SDK.

Represents any valid JSON value (serde_json::Value in Rust, bare {} in JSON Schema).
"""
from __future__ import annotations

from typing import Union

from typing_extensions import TypeAliasType

JsonValue = TypeAliasType(
    "JsonValue",
    Union[str, int, float, bool, None, list["JsonValue"], dict[str, "JsonValue"]],
)
