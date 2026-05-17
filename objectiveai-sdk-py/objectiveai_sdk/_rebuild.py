"""Global forward reference resolver.

Imports all generated modules and rebuilds every Pydantic model with a
shared namespace so that inherited and cross-module forward references
resolve correctly. Called lazily on first SDK attribute access.
"""

_rebuilt = False


def ensure_rebuilt():
    global _rebuilt
    if _rebuilt:
        return
    _rebuilt = True

    import importlib
    import sys

    from pydantic import BaseModel, RootModel

    from objectiveai_sdk._module_list import MODULES

    # Load all generated modules
    for mod_path in MODULES:
        importlib.import_module(mod_path)

    # Collect all models and their modules
    models = []
    for mod_path in MODULES:
        mod = sys.modules.get(mod_path)
        if mod is None:
            continue
        for obj in vars(mod).values():
            if (
                isinstance(obj, type)
                and issubclass(obj, BaseModel)
                and obj not in (BaseModel, RootModel)
                and hasattr(obj, "__pydantic_complete__")
            ):
                models.append((obj, mod))

    # Build a fallback namespace for inherited forward refs.
    # Contains all module globals merged together. Only used for names
    # NOT found in a model's own module globals.
    fallback_ns = {}
    for mod_path in MODULES:
        mod = sys.modules.get(mod_path)
        if mod is None:
            continue
        fallback_ns.update(vars(mod))

    # Rebuild each model. Start with the fallback (for inherited refs),
    # then overlay the model's own module globals (which take precedence
    # for names defined/imported in that module — handles collisions like
    # multiple types named "Task" across different packages).
    for cls, mod in models:
        ns = dict(fallback_ns)
        ns.update(vars(mod))
        cls.model_rebuild(_types_namespace=ns)
