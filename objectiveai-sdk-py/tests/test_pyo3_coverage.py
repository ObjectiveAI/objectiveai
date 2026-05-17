"""
PyO3 function coverage test: every function exported by objectiveai-sdk-rs-pyo3
must have a corresponding Python wrapper in a pyo3.py file at the path
matching the type it operates on.

Requirements:
1. Function names are identical to the Rust PyO3 function names (snake_case).
2. The Python pyo3.py file path mirrors the Rust type's module path:
   e.g. agent_completion_chunk_merged → objectiveai_sdk/agent/completions/response/streaming/pyo3.py
3. Every function must also be importable from its package __init__.py.
"""

import importlib
import os
import re
from pathlib import Path

import pytest

PYO3_RS = Path(__file__).resolve().parent.parent.parent / "objectiveai-sdk-rs-pyo3" / "src" / "lib.rs"
PY_SRC = Path(__file__).resolve().parent.parent / "objectiveai_sdk"

# ---------------------------------------------------------------------------
# Map each pyo3 function to the Python module path where its pyo3.py should live
# ---------------------------------------------------------------------------

# Functions grouped by the module path of the type they operate on.
FUNCTION_TO_MODULE: dict[str, str] = {
    # Validation & ID
    "validate_agent": "agent",
    "validate_swarm": "swarm",
    "prompt_id": "agent.completions.message",
    "vector_response_id": "agent.completions.message",

    # Function input/compilation
    "validate_function_input": "functions",
    "compile_function_tasks": "functions",
    "compile_function_output_length": "functions",
    "compile_function_input_split": "functions",
    "compile_function_input_merge": "functions",

    # Field validation
    "check_vector_fields": "functions.check",
    "check_scalar_fields": "functions.check",

    # Alpha function validation
    "alpha_check_leaf_scalar_function": "functions.alpha_scalar",
    "alpha_check_branch_scalar_function": "functions.alpha_scalar",
    "alpha_check_leaf_vector_function": "functions.alpha_vector",
    "alpha_check_branch_vector_function": "functions.alpha_vector",

    # Agent completion chunk operations
    "agent_completion_chunk_merged": "agent.completions.response.streaming",
    "agent_completion_chunk_normalized": "agent.completions.response.streaming",
    "agent_completion_chunk_to_unary": "agent.completions.response.streaming",
    "normalize_agent_completion_for_tests": "agent.completions.response.streaming",
    "generate_agent_completion_chunk": "agent.completions.response.streaming",

    # Vector completion chunk operations
    "vector_completion_chunk_merged": "vector.completions.response.streaming",
    "vector_completion_chunk_normalized": "vector.completions.response.streaming",
    "vector_completion_chunk_to_unary": "vector.completions.response.streaming",
    "normalize_vector_completion_for_tests": "vector.completions.response.streaming",
    "generate_vector_completion_chunk": "vector.completions.response.streaming",

    # Function execution chunk operations
    "function_execution_chunk_merged": "functions.executions.response.streaming",
    "function_execution_chunk_normalized": "functions.executions.response.streaming",
    "function_execution_chunk_to_unary": "functions.executions.response.streaming",
    "normalize_function_execution_for_tests": "functions.executions.response.streaming",
    "generate_function_execution_chunk": "functions.executions.response.streaming",

    # Function invention chunk operations
    "function_invention_chunk_merged": "functions.inventions.response.streaming",
    "function_invention_chunk_normalized": "functions.inventions.response.streaming",
    "function_invention_chunk_to_unary": "functions.inventions.response.streaming",
    "normalize_function_invention_for_tests": "functions.inventions.response.streaming",
    "generate_function_invention_chunk": "functions.inventions.response.streaming",

    # Function invention recursive chunk operations
    "function_invention_recursive_chunk_merged": "functions.inventions.recursive.response.streaming",
    "function_invention_recursive_chunk_normalized": "functions.inventions.recursive.response.streaming",
    "function_invention_recursive_chunk_to_unary": "functions.inventions.recursive.response.streaming",
    "normalize_function_invention_recursive_for_tests": "functions.inventions.recursive.response.streaming",
    "generate_function_invention_recursive_chunk": "functions.inventions.recursive.response.streaming",

    # Laboratory execution chunk operations
    "laboratory_execution_chunk_merged": "laboratories.executions.response.streaming",
    "laboratory_execution_chunk_normalized": "laboratories.executions.response.streaming",
    "laboratory_execution_chunk_to_unary": "laboratories.executions.response.streaming",
    "normalize_laboratory_execution_for_tests": "laboratories.executions.response.streaming",
    "generate_laboratory_execution_chunk": "laboratories.executions.response.streaming",

    # Function profile computation chunk operations
    "function_profile_computation_chunk_merged": "functions.profiles.computations.response.streaming",
    "function_profile_computation_chunk_normalized": "functions.profiles.computations.response.streaming",
    "function_profile_computation_chunk_to_unary": "functions.profiles.computations.response.streaming",
    "normalize_function_profile_computation_for_tests": "functions.profiles.computations.response.streaming",
    "generate_function_profile_computation_chunk": "functions.profiles.computations.response.streaming",
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def extract_pyo3_functions() -> list[str]:
    """Extract all function names registered in the pyo3 module."""
    content = PYO3_RS.read_text(encoding="utf-8")
    return re.findall(r"m\.add_function\(wrap_pyfunction!\((\w+),", content)


def find_pyo3_py_files() -> dict[str, list[str]]:
    """Find all pyo3.py files and extract their function names.

    Returns a map from dot-separated module path to list of function names.
    """
    result: dict[str, list[str]] = {}
    for dirpath, _, filenames in os.walk(PY_SRC):
        if "pyo3.py" in filenames:
            rel = os.path.relpath(dirpath, PY_SRC)
            if rel == ".":
                mod_path = ""
            else:
                mod_path = rel.replace(os.sep, ".")
            content = (Path(dirpath) / "pyo3.py").read_text(encoding="utf-8")
            fns = re.findall(r"(?:async\s+)?def\s+(\w+)", content)
            result[mod_path] = fns
    return result


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestPyo3Coverage:
    def test_all_pyo3_functions_are_mapped(self):
        """Every pyo3 exported function must have an entry in FUNCTION_TO_MODULE."""
        pyo3_fns = extract_pyo3_functions()
        unmapped = [fn for fn in pyo3_fns if fn not in FUNCTION_TO_MODULE]
        assert unmapped == [], f"Unmapped pyo3 functions: {unmapped}"

    def test_every_pyo3_function_has_python_wrapper(self):
        """Every pyo3 function must have a pyo3_-prefixed Python function in pyo3.py."""
        pyo3_py_files = find_pyo3_py_files()
        errors: list[str] = []

        for fn_name, mod_path in FUNCTION_TO_MODULE.items():
            py_fns = pyo3_py_files.get(mod_path, [])
            expected = f"pyo3_{fn_name}"
            if expected not in py_fns:
                errors.append(
                    f"{mod_path}: missing function \"{expected}\" in pyo3.py"
                )

        assert errors == [], (
            "Missing Python wrappers:\n  " + "\n  ".join(errors)
        )

    def test_every_python_pyo3_function_has_rust_counterpart(self):
        """Every Python pyo3.py function must have a corresponding Rust export."""
        pyo3_py_files = find_pyo3_py_files()
        pyo3_fns = set(extract_pyo3_functions())
        errors: list[str] = []

        for mod_path, py_fns in pyo3_py_files.items():
            for fn_name in py_fns:
                if fn_name.startswith("_"):
                    continue
                # Strip pyo3_ prefix to find the Rust counterpart
                rust_name = fn_name.removeprefix("pyo3_")
                if rust_name not in pyo3_fns:
                    errors.append(
                        f"{mod_path}: unexpected function \"{fn_name}\" "
                        f"has no pyo3 counterpart"
                    )

        assert errors == [], (
            "Extra Python functions without pyo3 counterpart:\n  "
            + "\n  ".join(errors)
        )

    def test_python_functions_have_pyo3_prefix(self):
        """Every public function in pyo3.py must be prefixed with pyo3_."""
        pyo3_py_files = find_pyo3_py_files()
        errors: list[str] = []

        for mod_path, py_fns in pyo3_py_files.items():
            for fn_name in py_fns:
                if fn_name.startswith("_"):
                    continue
                if not fn_name.startswith("pyo3_"):
                    errors.append(
                        f"{mod_path}: function \"{fn_name}\" missing pyo3_ prefix"
                    )

        assert errors == [], (
            "Functions missing pyo3_ prefix:\n  " + "\n  ".join(errors)
        )

    def test_functions_importable_from_package_init(self):
        """Every pyo3.py function must be importable from its package."""
        pyo3_py_files = find_pyo3_py_files()
        errors: list[str] = []

        for mod_path, py_fns in pyo3_py_files.items():
            package_path = f"objectiveai_sdk.{mod_path}" if mod_path else "objectiveai_sdk"

            try:
                package = importlib.import_module(package_path)
            except ImportError as e:
                errors.append(f"{mod_path}: cannot import package: {e}")
                continue

            for fn_name in py_fns:
                if fn_name.startswith("_"):
                    continue
                if not hasattr(package, fn_name):
                    errors.append(
                        f"{mod_path}: function \"{fn_name}\" not importable "
                        f"from {package_path}"
                    )

        assert errors == [], (
            "Functions not importable from package __init__:\n  "
            + "\n  ".join(errors)
        )
