def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


from objectiveai_sdk.swarm.http import *  # noqa: F401, F403, E402
from objectiveai_sdk.swarm.pyo3 import *  # noqa: F401, F403, E402
