"""
Roundtrip test: Pydantic model → JSON Schema must exactly match the original
objectiveai-json-schema/ files, proving no information is lost during the
Pydantic code generation.

RULES FOR THIS FILE
===================

1. This test code is FORBIDDEN from reading or deserializing the original
   JSON schema files. Doing so would amount to cheating — the whole point
   is that schemas must be reconstructible entirely from the generated
   Pydantic types.

2. The only things imported from the harness are:
   - ALL_TITLES: the set of schema title strings (metadata, not content)
   - assert_schema_matches(title, dict): the strict equality check

3. To make tests pass, the assistant is allowed to modify:
   - This test file (conversion / normalization logic)
   - The auto-generation script (scripts/install_pydantic.py)

4. The assistant is FORBIDDEN from modifying:
   - The harness (test_pydantic_roundtrip_harness.py)
   - The original JSON schemas (objectiveai-json-schema/*.json)

5. This test MUST be entirely generic. It must not contain any
   schema-specific logic, hardcoded titles, special cases, or
   conditional branches for particular types. It must work unchanged
   even if all existing JSON schemas are removed and replaced with
   entirely new ones. The only schema-aware code lives in the
   auto-generation script.

This is an information-loss and reconstructibility test.
"""

import importlib
import sys
import typing
from pathlib import Path
from typing import Any, Union, get_args, get_origin

import pytest
from pydantic import BaseModel, RootModel
from pydantic.fields import FieldInfo
from pydantic_core import PydanticUndefined

from objectiveai_sdk.json_value import JsonValue
from .test_pydantic_roundtrip_harness import ALL_TITLES, assert_schema_matches

# Import helpers from the generator so the test stays in sync automatically.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import install_pydantic  # noqa: E402
from install_pydantic import (  # noqa: E402
    compute_global_class_names,
    title_to_class_name,
)
from install_pydantic import title_to_path as _title_to_path  # noqa: E402


# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------


# Compute global class names (handles within-file collisions).
GLOBAL_CLASS_NAMES = compute_global_class_names(ALL_TITLES)


def title_to_module_and_name(title: str) -> tuple[str, str]:
    """Map a schema title to (package_path, class_name).

    Imports from the package (via __getattr__), not the file directly,
    matching how SDK users import types.
    """
    dir_path, _file_name = _title_to_path(title)
    if dir_path:
        module_path = "objectiveai_sdk." + dir_path.replace("/", ".")
    else:
        module_path = "objectiveai_sdk"
    class_name = GLOBAL_CLASS_NAMES.get(title, title_to_class_name(title))
    return module_path, class_name


# ---------------------------------------------------------------------------
# Load all Pydantic types
# ---------------------------------------------------------------------------



def load_pydantic_types() -> dict[str, Any]:
    """Import all generated Pydantic types.

    Imports from packages (not files) to match SDK user behavior.
    Forward reference resolution is handled automatically by
    objectiveai._rebuild.ensure_rebuilt(), triggered lazily via __getattr__.
    """
    types: dict[str, Any] = {}
    for title in ALL_TITLES:
        module_path, class_name = title_to_module_and_name(title)
        try:
            mod = importlib.import_module(module_path)
            cls = getattr(mod, class_name)
            types[title] = cls
        except (ImportError, AttributeError) as e:
            types[title] = e

    return types


pydantic_types = load_pydantic_types()

# Build reverse mapping: class object id → schema title.
# Uses object identity (id) to handle class name collisions across packages.
_class_id_to_title: dict[int, str] = {}
for _t, _cls in pydantic_types.items():
    if not isinstance(_cls, Exception):
        _class_id_to_title[id(_cls)] = _t


# ---------------------------------------------------------------------------
# Custom Pydantic → JSON Schema converter
# ---------------------------------------------------------------------------


def _get_extra_setting(cls: type) -> str | None:
    """Get the 'extra' setting from model_config."""
    config = getattr(cls, "model_config", None)
    if config and isinstance(config, dict):
        return config.get("extra")
    return None


def _is_known_type(tp: Any) -> str | None:
    """If tp is a known Pydantic type (in ALL_TITLES), return its title.

    Uses object identity to avoid class name collisions across packages.
    """
    if isinstance(tp, type):
        return _class_id_to_title.get(id(tp))
    return None


def _is_none_type(tp: Any) -> bool:
    return tp is type(None)


def _is_nullable_type(tp: Any) -> bool:
    """Check if tp is Optional[X] (Union[X, None])."""
    origin = get_origin(tp)
    if origin is Union:
        return any(_is_none_type(a) for a in get_args(tp))
    return False


def _unwrap_annotated(tp: Any) -> tuple[Any, list[Any]]:
    """Unwrap Annotated[X, ...] → (X, [metadata...])."""
    origin = get_origin(tp)
    if origin is typing.Annotated:
        args = get_args(tp)
        return args[0], list(args[1:])
    return tp, []


def _extract_annotated_constraints(metadata: list[Any]) -> dict:
    """Extract JSON Schema constraints from Annotated metadata (FieldInfo, Ge, Le, etc.)."""
    result: dict = {}
    for m in metadata:
        if isinstance(m, FieldInfo):
            # Check FieldInfo's own metadata list
            for mm in (m.metadata or []):
                if hasattr(mm, "ge") and mm.ge is not None:
                    result["minimum"] = mm.ge
                if hasattr(mm, "le") and mm.le is not None:
                    result["maximum"] = mm.le
                if hasattr(mm, "pattern") and mm.pattern is not None:
                    result["pattern"] = mm.pattern
                # List length bounds (Field(min_length/max_length) on a
                # list — e.g. the fixed-2 `[key, value]` pairs).
                if hasattr(mm, "min_length") and mm.min_length is not None:
                    result["minItems"] = mm.min_length
                if hasattr(mm, "max_length") and mm.max_length is not None:
                    result["maxItems"] = mm.max_length
            # Check json_schema_extra
            extra = m.json_schema_extra
            if isinstance(extra, dict):
                if "format" in extra:
                    result["format"] = extra["format"]
                if "pattern" in extra:
                    result["pattern"] = extra["pattern"]
        else:
            # Direct constraint objects (Ge, Le, MinLen, MaxLen, etc.)
            if hasattr(m, "ge") and m.ge is not None:
                result["minimum"] = m.ge
            if hasattr(m, "le") and m.le is not None:
                result["maximum"] = m.le
            if hasattr(m, "pattern") and m.pattern is not None:
                result["pattern"] = m.pattern
            if hasattr(m, "min_length") and m.min_length is not None:
                result["minItems"] = m.min_length
            if hasattr(m, "max_length") and m.max_length is not None:
                result["maxItems"] = m.max_length
    return result


def convert_type(tp: Any, root_title: str) -> dict:
    """Convert a Python type annotation to JSON Schema.

    Handles Annotated wrappers, extracting constraints from Field metadata.
    """
    # Unwrap Annotated[T, Field(...)]
    base_tp, metadata = _unwrap_annotated(tp)
    constraints = _extract_annotated_constraints(metadata)

    result = _convert_type_inner(base_tp, root_title)
    result.update(constraints)
    return result


def _convert_type_inner(tp: Any, root_title: str) -> dict:
    """Inner conversion without Annotated unwrapping."""
    if _is_none_type(tp):
        return {"type": "null"}

    # Check if it's a known type → emit $ref
    known = _is_known_type(tp)
    if known:
        return {"$ref": known}

    # RootModel subclass → check for metadata, then unwrap
    if isinstance(tp, type) and issubclass(tp, RootModel) and tp is not RootModel:
        return _convert_root_model(tp, root_title)

    # BaseModel subclass → object with properties
    if isinstance(tp, type) and issubclass(tp, BaseModel) and tp is not BaseModel:
        return _convert_base_model(tp, root_title)

    # Primitive types
    if tp is str:
        return {"type": "string"}
    if tp is int:
        return {"type": "integer"}
    if tp is float:
        return {"type": "number"}
    if tp is bool:
        return {"type": "boolean"}

    # Native types with JSON Schema format
    from datetime import datetime as _datetime
    from uuid import UUID as _UUID
    if tp is _datetime:
        return {"type": "string", "format": "date-time"}
    if tp is _UUID:
        return {"type": "string", "format": "uuid"}

    # object / JsonValue (bare schema — any JSON value)
    if tp is object or tp is JsonValue:
        return {}

    origin = get_origin(tp)
    args = get_args(tp)

    # Union
    if origin is Union:
        return _convert_union(list(args), root_title)

    # list
    if origin is list:
        result: dict = {"type": "array"}
        if args:
            result["items"] = convert_type(args[0], root_title)
        return result

    # dict
    if origin is dict:
        if args and len(args) == 2:
            val_type = args[1]
            if val_type is object or val_type is JsonValue:
                return {"type": "object", "additionalProperties": True}
            val_schema = convert_type(val_type, root_title)
            return {"type": "object", "additionalProperties": val_schema}
        return {"type": "object"}

    # Literal
    if origin is typing.Literal:
        values = list(args)
        result: dict = {}
        # Infer type from literal values
        if values and all(isinstance(v, str) for v in values):
            result["type"] = "string"
        elif values and all(isinstance(v, int) for v in values):
            result["type"] = "integer"
        result["enum"] = values
        return result

    return {}


def _convert_union(args: list[Any], root_title: str) -> dict:
    """Convert a Union type to anyOf schema."""
    none_args = [a for a in args if _is_none_type(a)]
    non_none_args = [a for a in args if not _is_none_type(a)]

    if none_args and len(non_none_args) == 1:
        inner = _convert_union_member(non_none_args[0], root_title)
        return {"anyOf": [inner, {"type": "null"}]}

    variants = [_convert_union_member(a, root_title) for a in args]
    return {"anyOf": variants}


def _get_variant_title(tp: Any) -> str | None:
    """Read _variant_title from a type's model_config json_schema_extra."""
    config = getattr(tp, "model_config", None)
    if config and isinstance(config, dict):
        extra = config.get("json_schema_extra")
        if isinstance(extra, dict):
            return extra.get("_variant_title")
    return None


def _get_variant_outer_object(tp: Any) -> bool:
    """Read _outer_object marker — set by install_pydantic.py whenever
    the source schema stamped `type: "object"` alongside the variant's
    `$ref` (struct-flattens-untagged-enum / internally-tagged-enum cases).
    """
    config = getattr(tp, "model_config", None)
    if config and isinstance(config, dict):
        extra = config.get("json_schema_extra")
        if isinstance(extra, dict):
            return bool(extra.get("_outer_object"))
    return False


def _convert_union_member(tp: Any, root_title: str) -> dict:
    """Convert a single Union member to a JSON Schema dict.

    For inline variant types (not in ALL_TITLES), includes description
    from docstring and converts the type inline.
    """
    if _is_none_type(tp):
        return {"type": "null"}

    # Read variant title metadata (emitted by install_pydantic.py)
    variant_title = _get_variant_title(tp)

    known = _is_known_type(tp)
    if known:
        result: dict = {}
        if variant_title:
            result["title"] = variant_title
        result["$ref"] = known
        return result

    # Inline variant type — include description from docstring
    if isinstance(tp, type) and issubclass(tp, (BaseModel, RootModel)):
        result = {}
        if variant_title:
            result["title"] = variant_title
        doc = getattr(tp, "__doc__", None)
        if doc:
            result["description"] = doc
        if _get_variant_outer_object(tp):
            result["type"] = "object"
        if issubclass(tp, RootModel):
            inner = _convert_root_model(tp, root_title)
        else:
            inner = _convert_base_model(tp, root_title)
        result.update(inner)
        return result

    result = convert_type(tp, root_title)
    if variant_title:
        result = {"title": variant_title, **result}
    return result


def _convert_root_model(cls: type, root_title: str) -> dict:
    """Convert a RootModel subclass to JSON Schema."""
    # Check for expanded $ref (union that was expanded for inheritance).
    # Reconstruct as $ref + local properties.
    config = getattr(cls, "model_config", None)
    if config and isinstance(config, dict):
        extra = config.get("json_schema_extra")
        if isinstance(extra, dict) and "_expanded_ref" in extra:
            return _convert_expanded_ref_model(cls, root_title, extra["_expanded_ref"])

    # Plain RootModel — unwrap root type and extract field constraints
    fields = cls.model_fields
    if "root" not in fields:
        return {}
    field_info = fields["root"]
    root_type = field_info.annotation
    result: dict = {}

    # Include description from docstring (for inline generated types)
    doc = getattr(cls, "__doc__", None)
    if doc and not _is_known_type(cls):
        result["description"] = doc

    result.update(convert_type(root_type, root_title))

    # Extract constraints from field_info.metadata (Pydantic unwraps Annotated)
    for m in (field_info.metadata or []):
        if hasattr(m, "ge") and m.ge is not None:
            result["minimum"] = m.ge
        if hasattr(m, "le") and m.le is not None:
            result["maximum"] = m.le
        if hasattr(m, "pattern") and m.pattern is not None:
            result["pattern"] = m.pattern
    fi_extra = field_info.json_schema_extra
    if isinstance(fi_extra, dict):
        if "format" in fi_extra:
            result["format"] = fi_extra["format"]
        if "pattern" in fi_extra:
            result["pattern"] = fi_extra["pattern"]

    return result


def _convert_expanded_ref_model(cls: type, root_title: str, expanded_ref: str) -> dict:
    """Convert an expanded union $ref back to $ref + local properties.

    The expanded union has variants that inherit from the original union's
    inner types with local properties added. We reconstruct the original
    schema shape: {"type": "object", "$ref": "...", "properties": {...}}.
    """
    result: dict = {}

    # Include description from the class docstring
    doc = getattr(cls, "__doc__", None)
    if doc:
        result["description"] = doc

    result["type"] = "object"
    result["$ref"] = expanded_ref

    # Get the list of local property names from metadata
    config = getattr(cls, "model_config", None)
    extra = config.get("json_schema_extra", {}) if config else {}
    local_prop_names = extra.get("_expanded_ref_props")

    # Find the local properties from the first variant
    variants = _find_variant_types(cls)
    if variants and local_prop_names:
        first_variant = variants[0]
        all_props = _convert_properties(first_variant, root_title)
        # Filter to only local properties, preserving order from schema
        local_props = {}
        for name in local_prop_names:
            if name in all_props:
                local_props[name] = all_props[name]
        if local_props:
            result["properties"] = local_props
    # No local_prop_names means no local properties — just $ref, no properties block

    return result


def _get_root_annotation(cls: type) -> Any:
    """Get the root field type annotation from a RootModel."""
    fields = cls.model_fields
    if "root" in fields:
        return fields["root"].annotation
    return object


def _find_variant_types(cls: type) -> list[type]:
    """Find variant types for a class from its Union root annotation."""
    fields = getattr(cls, "model_fields", {})
    if "root" not in fields:
        return []
    root_type = fields["root"].annotation
    args = get_args(root_type)
    if not args:
        return []
    return [a for a in args if not _is_none_type(a)]


def _get_ref_base(cls: type) -> str | None:
    """If cls inherits from a known type (in ALL_TITLES), return its title."""
    for base in cls.__mro__[1:]:
        if base in (BaseModel, RootModel, object):
            continue
        known = _is_known_type(base)
        if known:
            return known
    return None


def _convert_base_model(cls: type, root_title: str) -> dict:
    """Convert a BaseModel subclass to a JSON Schema object."""
    result: dict = {"type": "object"}

    # Check if this class inherits from a known $ref type (inheritance pattern).
    # If so, emit $ref and only the locally-defined properties (not inherited ones).
    ref_base = _get_ref_base(cls)
    if ref_base:
        result["$ref"] = ref_base
        # Only emit properties defined directly on this class (not inherited)
        local_properties = _convert_local_properties(cls, root_title)
        if local_properties:
            result["properties"] = local_properties
        return result

    # Discover variant types by naming convention (flatten pattern)
    variants = _find_variant_types(cls)
    if len(variants) == 1:
        # Single variant → emit its schema directly (e.g. $ref)
        variant_schema = _convert_union_member(variants[0], root_title)
        result.update(variant_schema)
    elif len(variants) > 1:
        # Multiple variants → emit anyOf
        result["anyOf"] = [_convert_union_member(v, root_title) for v in variants]

    properties = _convert_properties(cls, root_title)
    if properties:
        result["properties"] = properties

    # additionalProperties: extra='forbid' → false, extra='allow' → true
    extra_setting = _get_extra_setting(cls)
    if extra_setting == "forbid":
        result["additionalProperties"] = False
    elif extra_setting == "allow":
        result["additionalProperties"] = True

    return result


def _convert_local_properties(cls: type, root_title: str) -> dict:
    """Convert only locally-defined fields (including redeclared overrides
    of inherited fields, e.g. a `type` discriminator narrowed from `ErrorType`
    to `Literal["error"]`) to JSON Schema properties.

    Uses `cls.__annotations__`, which contains only annotations directly
    declared on this class (not inherited), so a redeclaration is treated
    as local rather than filtered out as "inherited."
    """
    local_annotations = getattr(cls, "__annotations__", {})

    properties: dict = {}
    for field_name, field_info in cls.model_fields.items():
        if field_name not in local_annotations:
            continue
        prop_name = field_info.alias if field_info.alias else field_name
        tp = field_info.annotation
        prop_schema = _convert_property(tp, field_info, root_title)
        properties[prop_name] = prop_schema
    return properties


def _convert_properties(cls: type, root_title: str) -> dict:
    """Convert BaseModel fields to JSON Schema properties."""
    properties: dict = {}
    fields = cls.model_fields

    for field_name, field_info in fields.items():
        prop_name = field_info.alias if field_info.alias else field_name
        tp = field_info.annotation
        prop_schema = _convert_property(tp, field_info, root_title)
        properties[prop_name] = prop_schema

    return properties


def _convert_property(tp: Any, field_info: FieldInfo, root_title: str) -> dict:
    """Convert a single property (type + field info) to JSON Schema."""
    result: dict = {}

    # Description from Field
    if field_info.description:
        result["description"] = field_info.description

    # Check if nullability was inherited from a nested anyOf.
    # If so, strip the Optional wrapper — the inner type already contains null.
    fi_extra = field_info.json_schema_extra
    inherited_nullable = isinstance(fi_extra, dict) and fi_extra.get("_nullable") is False
    if inherited_nullable and _is_nullable_type(tp):
        # Unwrap Optional[X] → X
        non_none = [a for a in get_args(tp) if not _is_none_type(a)]
        tp = non_none[0] if len(non_none) == 1 else Union[tuple(non_none)]

    # Convert the type annotation to JSON Schema
    type_schema = convert_type(tp, root_title)

    # Extract constraints from field_info.metadata (for non-nullable props
    # where Pydantic merges Annotated Field into field_info.metadata)
    fi_constraints: dict = {}
    for m in (field_info.metadata or []):
        if hasattr(m, "ge") and m.ge is not None:
            fi_constraints["minimum"] = m.ge
        if hasattr(m, "le") and m.le is not None:
            fi_constraints["maximum"] = m.le
        if hasattr(m, "pattern") and m.pattern is not None:
            fi_constraints["pattern"] = m.pattern
    fi_extra = field_info.json_schema_extra
    if isinstance(fi_extra, dict):
        if "format" in fi_extra:
            fi_constraints["format"] = fi_extra["format"]
        if "pattern" in fi_extra:
            fi_constraints["pattern"] = fi_extra["pattern"]
        if "additionalProperties" in fi_extra:
            fi_constraints["additionalProperties"] = fi_extra["additionalProperties"]
    # Place constraints correctly:
    # - For nullable types: constraints should go inside the non-null anyOf variant
    # - For non-nullable types: constraints go directly on the property
    if fi_constraints:
        if "anyOf" in type_schema:
            # Nullable: overlay constraints on the non-null variant
            for variant in type_schema["anyOf"]:
                if variant.get("type") != "null":
                    variant.update(fi_constraints)
                    break
        else:
            type_schema.update(fi_constraints)

    result.update(type_schema)

    # Default value — but don't emit "default: null" for nullable fields
    # since that's just the implicit Optional default, not an explicit schema default
    if field_info.default is not PydanticUndefined:
        if field_info.default is None and (_is_nullable_type(field_info.annotation) or inherited_nullable):
            pass  # Suppress implicit default: null for nullable fields
        else:
            result["default"] = field_info.default

    # omitempty is a property-level attribute, not a type constraint
    if isinstance(fi_extra, dict) and fi_extra.get("omitempty") is True:
        result["omitempty"] = True

    return result


def convert_top_level(cls: Any, title: str) -> dict:
    """Convert a Pydantic type to a complete JSON Schema with title and description."""
    result: dict = {"title": title}

    # Get description from docstring
    doc = getattr(cls, "__doc__", None)
    if doc:
        result["description"] = doc

    # Convert the type itself
    if isinstance(cls, type) and issubclass(cls, BaseModel) and not issubclass(cls, RootModel):
        inner = _convert_base_model(cls, title)
        result.update(inner)
    elif isinstance(cls, type) and issubclass(cls, RootModel):
        inner = _convert_root_model(cls, title)
        result.update(inner)
    else:
        inner = convert_type(cls, title)
        result.update(inner)

    return result


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("title", sorted(ALL_TITLES))
def test_roundtrip(title: str) -> None:
    """Verify Pydantic model → JSON Schema exactly matches the original."""
    pydantic_type = pydantic_types[title]

    if isinstance(pydantic_type, Exception):
        pytest.fail(f"Failed to import Pydantic type for '{title}': {pydantic_type}")

    converted = convert_top_level(pydantic_type, title)
    assert_schema_matches(title, converted)
