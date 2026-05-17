def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


import objectiveai_sdk.agent.completions.message.assistant_tool_call_delta_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.message.assistant_tool_call_function_delta_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.message.rich_content_methods  # noqa: F401, E402
from objectiveai_sdk.agent.completions.message.pyo3 import *  # noqa: F401, F403, E402
