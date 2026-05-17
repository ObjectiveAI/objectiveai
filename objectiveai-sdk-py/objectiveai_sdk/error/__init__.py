def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


from objectiveai_sdk.error.fetch_error import ObjectiveAIFetchError, _is_response_error  # noqa: F401, E402
from objectiveai_sdk.error.http import *  # noqa: F401, F403, E402
