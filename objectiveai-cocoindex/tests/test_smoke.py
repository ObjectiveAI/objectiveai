def test_cocoindex_importable():
    import cocoindex  # noqa: F401


def test_objectiveai_importable():
    import objectiveai  # noqa: F401


def test_package_importable():
    import objectiveai_cocoindex  # noqa: F401


def test_public_surface():
    from objectiveai_cocoindex import (  # noqa: F401
        Function,
        set_default_client,
        FunctionSource,
        ProfileSource,
        RemoteFunction,
        InlineFunction,
        RemoteProfile,
        InlineProfile,
        ObjectiveAIExecutionError,
    )
