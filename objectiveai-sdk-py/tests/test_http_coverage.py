"""
HTTP function coverage test: every Rust http.rs file must have a
corresponding Python http.py file at the same path within objectiveai/,
exporting functions whose names match the Rust function names in snake_case.

Requirements:
1. Function names are identical to Rust (already snake_case).
2. The Python file path mirrors the Rust module path:
   Rust:   objectiveai-rs/src/agent/completions/http.rs
   Python: objectiveai/agent/completions/http.py
3. Every function must also be importable from its package __init__.py.

Streaming/unary pairs (e.g. create_foo_unary / create_foo_streaming)
map to a single Python function (e.g. create_foo).
"""

import importlib
import os
import re
from pathlib import Path

import pytest

RUST_SRC = Path(__file__).resolve().parent.parent.parent / "objectiveai-rs" / "src"
PY_SRC = Path(__file__).resolve().parent.parent / "objectiveai"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def find_http_files(root: Path, ext: str) -> dict[str, str]:
    """Walk a source tree looking for files named http.{ext}.

    Returns a map from dot-separated module path to file contents.
      src/agent/completions/http.rs  →  "agent.completions"
    """
    result: dict[str, str] = {}
    for dirpath, _, filenames in os.walk(root):
        if f"http.{ext}" in filenames:
            rel = os.path.relpath(dirpath, root)
            if rel == ".":
                module_path = ""
            else:
                module_path = rel.replace(os.sep, ".")
            content = (Path(dirpath) / f"http.{ext}").read_text(encoding="utf-8")
            result[module_path] = content
    return result


def extract_rust_functions(content: str) -> list[str]:
    """Extract `pub async fn <name>` from Rust source."""
    return re.findall(r"pub\s+async\s+fn\s+(\w+)", content)


def extract_python_functions(content: str) -> list[str]:
    """Extract top-level `def <name>` and `async def <name>` from Python source."""
    return list(set(re.findall(r"(?:async\s+)?def\s+(\w+)", content)))


def strip_streaming_suffix(name: str) -> str:
    """Strip _unary / _streaming suffix to get the base function name."""
    return re.sub(r"_(unary|streaming)$", "", name)


# ---------------------------------------------------------------------------
# Discover Rust http files
# ---------------------------------------------------------------------------

rust_http_files = find_http_files(RUST_SRC, "rs")
py_http_files = find_http_files(PY_SRC, "py")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestHttpCoverage:
    def test_every_rust_http_has_python_http(self):
        """Every Rust http.rs must have a corresponding Python http.py."""
        missing = [
            mod_path for mod_path in rust_http_files
            if mod_path not in py_http_files
        ]
        assert missing == [], f"Missing http.py for: {', '.join(missing)}"

    def test_every_rust_function_has_python_export(self):
        """Every Rust http function must have a corresponding Python function."""
        errors: list[str] = []

        for mod_path, rust_content in rust_http_files.items():
            py_content = py_http_files.get(mod_path)
            if py_content is None:
                continue

            rust_fns = extract_rust_functions(rust_content)
            py_fn_names = set(extract_python_functions(py_content))

            # Deduplicate streaming/unary pairs
            rust_base_names = sorted(set(
                strip_streaming_suffix(fn) for fn in rust_fns
            ))

            for rust_base in rust_base_names:
                if rust_base not in py_fn_names:
                    errors.append(
                        f"{mod_path}: expected function \"{rust_base}\" "
                        f"(from Rust)"
                    )

        assert errors == [], (
            "Missing Python functions:\n  " + "\n  ".join(errors)
        )

    def test_every_python_http_export_has_rust_counterpart(self):
        """Every Python http function must have a corresponding Rust function."""
        errors: list[str] = []

        for mod_path, py_content in py_http_files.items():
            rust_content = rust_http_files.get(mod_path)
            if rust_content is None:
                errors.append(f"{mod_path} (http.py exists without http.rs)")
                continue

            py_fns = extract_python_functions(py_content)
            rust_fns = extract_rust_functions(rust_content)
            expected_names = set(
                strip_streaming_suffix(fn) for fn in rust_fns
            )

            for py_fn in py_fns:
                if py_fn.startswith("_"):
                    continue  # skip private helpers
                if py_fn not in expected_names:
                    errors.append(
                        f"{mod_path}: unexpected function \"{py_fn}\" "
                        f"has no Rust counterpart"
                    )

        assert errors == [], (
            "Extra Python functions without Rust counterpart:\n  "
            + "\n  ".join(errors)
        )

    def test_functions_importable_from_package_init(self):
        """Every Python http function must be importable from its package."""
        errors: list[str] = []

        for mod_path, py_content in py_http_files.items():
            py_fns = [
                fn for fn in extract_python_functions(py_content)
                if not fn.startswith("_")
            ]

            if mod_path:
                package_path = "objectiveai." + mod_path
            else:
                package_path = "objectiveai"

            try:
                package = importlib.import_module(package_path)
            except ImportError as e:
                errors.append(f"{mod_path}: cannot import package: {e}")
                continue

            for fn_name in py_fns:
                if not hasattr(package, fn_name):
                    errors.append(
                        f"{mod_path}: function \"{fn_name}\" not importable "
                        f"from {package_path}"
                    )

        assert errors == [], (
            "Functions not importable from package __init__:\n  "
            + "\n  ".join(errors)
        )
