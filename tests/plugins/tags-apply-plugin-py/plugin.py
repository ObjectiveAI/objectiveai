"""E2E test fixture: a plugin written in Python, run by ``plugins run`` under
the objectiveai-sdk-py venv python as ``<venv python> ./plugin.py``. It uses the
Python SDK's in-process ``PluginCommandExecutor`` + the generated
``agents tags apply`` execute fn to mutate host state (apply a tag to a mock
agent) over the NDJSON command protocol, then emits one notification and exits.

Unlike the JS plugin it needs no explicit ``exit``: the executor's stdin reader
is a daemon thread, so once ``asyncio.run`` returns the interpreter shuts down
and the process ends — which closes the plugin's stdout and lets ``plugins run``
return.
"""
import asyncio
import sys
import warnings

# Importing the SDK emits pydantic UserWarnings (a generated `schema` field
# shadows BaseModel.schema). The plugin host turns ANY plugin stderr line into
# an error item (objectiveai-cli/src/command/plugins/run.rs), which would fail
# the run — so silence warnings before the import to keep stderr clean.
warnings.filterwarnings("ignore")

from objectiveai_sdk.cli.command.executor import PluginCommandExecutor
from objectiveai_sdk.cli.command.agents.tags.apply import execute
from objectiveai_sdk.cli.command.agents.tags.apply.request import Request

TAG = "py-plugin-applied-tag"


async def main() -> None:
    executor = PluginCommandExecutor.instance()
    await execute(
        executor,
        Request(
            name=TAG,
            path_type="agents/tags/apply",
            target={
                "by": "agent",
                "agent_spec": {"upstream": "mock", "output_mode": "instruction"},
            },
        ),
    )
    sys.stdout.write(f'{{"type":"notification","applied":"{TAG}"}}\n')
    sys.stdout.flush()


asyncio.run(main())
