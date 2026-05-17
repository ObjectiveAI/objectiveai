def __getattr__(name):
    from objectiveai_sdk._rebuild import ensure_rebuilt
    ensure_rebuilt()
    import importlib
    _generated = importlib.import_module(__name__ + '._generated')
    return getattr(_generated, name)


import objectiveai_sdk.agent.completions.response.completion_tokens_details_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.response.cost_details_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.response.logprobs_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.response.prompt_tokens_details_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.response.upstream_usage_methods  # noqa: F401, E402
import objectiveai_sdk.agent.completions.response.usage_methods  # noqa: F401, E402
