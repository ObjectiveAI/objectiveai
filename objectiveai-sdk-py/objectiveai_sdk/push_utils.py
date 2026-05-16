"""Shared helpers for push (streaming chunk accumulation)."""

from __future__ import annotations


def push_by_index(self_list: list, other_list: list) -> None:
    """Merge *other_list* into *self_list* by ``index`` field.

    Items with a matching index are merged via ``push()``.
    New indices are appended.
    """
    index_map: dict[int, int] = {}
    for pos, item in enumerate(self_list):
        idx = _get_index(item)
        if idx is not None:
            index_map[idx] = pos

    for other_item in other_list:
        idx = _get_index(other_item)
        if idx is not None and idx in index_map:
            self_list[index_map[idx]].push(other_item)
        else:
            self_list.append(other_item)
            if idx is not None:
                index_map[idx] = len(self_list) - 1


def push_option(obj: object, attr: str, other_val) -> None:
    """Conditionally merge an optional sub-object field.

    Both present → delegate to push(). Only other → adopt.
    Only self / neither → no assignment (field stays unset).
    """
    self_val = getattr(obj, attr)
    if self_val is not None and other_val is not None:
        self_val.push(other_val)
    elif other_val is not None:
        setattr(obj, attr, other_val)


def push_replace(obj: object, attr: str, other_val) -> None:
    """Replace field only if other is not None (latest wins)."""
    if other_val is not None:
        setattr(obj, attr, other_val)


def push_option_int(obj: object, attr: str, other_val: int | None) -> None:
    """Sum two optional ints. Only assigns if other is not None."""
    if other_val is None:
        return
    self_val = getattr(obj, attr)
    if self_val is not None:
        setattr(obj, attr, self_val + other_val)
    else:
        setattr(obj, attr, other_val)


def push_option_string(obj: object, attr: str, other_val: str | None) -> None:
    """Concatenate two optional strings. Only assigns if other is not None."""
    if other_val is None:
        return
    self_val = getattr(obj, attr)
    if self_val is not None:
        setattr(obj, attr, self_val + other_val)
    else:
        setattr(obj, attr, other_val)


def push_option_decimal(obj: object, attr: str, other_val) -> None:
    """Sum two optional decimals/floats. Only assigns if other is not None."""
    if other_val is None:
        return
    self_val = getattr(obj, attr)
    if self_val is not None:
        setattr(obj, attr, self_val + other_val)
    else:
        setattr(obj, attr, other_val)


def push_lazy_set_true(obj: object, attr: str, other_val: bool | None) -> None:
    """Set to True only if other is True. Never overwrites to False."""
    if other_val is True:
        setattr(obj, attr, True)


def _get_index(item):
    """Extract an integer index from a model (BaseModel or RootModel)."""
    from pydantic import RootModel
    if isinstance(item, RootModel):
        inner = item.root
        if isinstance(inner, RootModel):
            inner = inner.root
        return getattr(inner, "index", None)
    return getattr(item, "index", None)
