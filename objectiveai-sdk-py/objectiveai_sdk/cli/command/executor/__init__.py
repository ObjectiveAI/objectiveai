"""CLI command executors (async).

Mirror of ``objectiveai-sdk-js/src/cli/command/executor/``. The viewer executor
is intentionally absent — Python never hosts the Tauri viewer, so there are only
two: the binary executor (spawns the cli) and the in-process plugin executor.

The generated per-leaf ``execute`` functions accept one of these and ``async
for`` over its ``execute(request)``.
"""
from __future__ import annotations

from typing import Union

from objectiveai_sdk.cli.command.executor.binary import BinaryCommandExecutor
from objectiveai_sdk.cli.command.executor.plugin import PluginCommandExecutor

# Any of the two CLI command executors.
CommandExecutor = Union[BinaryCommandExecutor, PluginCommandExecutor]

__all__ = [
    "BinaryCommandExecutor",
    "PluginCommandExecutor",
    "CommandExecutor",
]
