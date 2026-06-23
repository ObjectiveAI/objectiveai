"""Shared utilities for push/merge fuzz tests.

Mirrors the JS merged.test.ts pattern:
1. Generate deterministic chunks via PyO3 (Rust arbitrary with seed)
2. Push via Python (Pydantic) + via PyO3 (Rust merged)
3. Compare with rounded floats for precision tolerance
"""
from __future__ import annotations

import math
from typing import Any


def rounded(value: Any) -> Any:
    """Round all floats to 8 significant figures for comparison.

    Mirrors mergeTestUtil.ts rounded(). Double-rounds through 12 digits
    first to normalize 1-ULP representation artifacts that can cause
    different rounding directions at the target precision.
    """
    if isinstance(value, bool):
        return value
    if isinstance(value, float):
        if value == 0 or not math.isfinite(value):
            return value
        return float(f"{float(f'{value:.12g}'):.8g}")
    if isinstance(value, list):
        return [rounded(v) for v in value]
    if isinstance(value, dict):
        return {k: rounded(v) for k, v in value.items()}
    return value
