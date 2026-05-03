"""PyO3 bindings for function operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_validate_function_input(function, input):
    """Validate function input against its schema."""
    return objectiveai_pyo3.validate_function_input(function, input)


def pyo3_compile_function_tasks(function, input):
    """Compile a function's task expressions for a given input."""
    return objectiveai_pyo3.compile_function_tasks(function, input)


def pyo3_compile_function_output_length(function, input):
    """Compute the expected output length for a vector function."""
    return objectiveai_pyo3.compile_function_output_length(function, input)


def pyo3_compile_function_input_split(function, input):
    """Compile the input_split expression to split input into sub-inputs."""
    return objectiveai_pyo3.compile_function_input_split(function, input)


def pyo3_compile_function_input_merge(function, input):
    """Compile the input_merge expression to merge sub-inputs back into one."""
    return objectiveai_pyo3.compile_function_input_merge(function, input)
