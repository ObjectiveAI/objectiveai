def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


import objectiveai_sdk.laboratories.executions.response.streaming.builder_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.laboratories.executions.response.streaming.evaluation_chunk_methods  # noqa: F401, E402
import objectiveai_sdk.laboratories.executions.response.streaming.laboratory_execution_chunk_methods  # noqa: F401, E402
from objectiveai_sdk.laboratories.executions.response.streaming.pyo3 import *  # noqa: F401, F403, E402
