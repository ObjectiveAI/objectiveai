def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


import objectiveai_sdk.functions.executions.response.streaming.function_execution_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.functions.executions.response.streaming.function_execution_task_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.functions.executions.response.streaming.reasoning_summary_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.functions.executions.response.streaming.task_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.functions.executions.response.streaming.vector_completion_task_chunk_methods  # noqa: F401, E402
from objectiveai_sdk.functions.executions.response.streaming.pyo3 import *  # noqa: F401, F403, E402
