def __getattr__(name):
    from objectiveai._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


from objectiveai_sdk.vector.completions.http import *  # noqa: F401, F403, E402
