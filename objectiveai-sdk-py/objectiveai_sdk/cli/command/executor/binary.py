"""Binary command executor — spawns the objectiveai CLI as a child process.

Port of ``objectiveai-sdk-js/src/cli/command/executor/binary.ts``. Async-only.

The request is serialized to JSON and passed via the cli's top-level
``--request`` flag; stdin is null, stdout is piped (yielded raw, line by line —
the caller's :class:`CliStream` discriminates errors vs responses), stderr is
inherited.
"""
from __future__ import annotations

import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any, AsyncIterator, Mapping, Optional


class BinaryCommandExecutor:
    """Spawn ``<objectiveai_dir>/bin/objectiveai[.exe] --request <json>`` and
    stream its stdout as parsed JSON values.

    :param objectiveai_dir: layout root; defaults to ``~/.objectiveai``.
    :param extra_env: environment variables layered onto the child's env.
    :param kill_on_drop: kill the child when the stream is closed early.
    :param detach: detach the child so it outlives the parent.
    """

    def __init__(
        self,
        *,
        objectiveai_dir: Optional[str] = None,
        extra_env: Optional[Mapping[str, str]] = None,
        kill_on_drop: bool = False,
        detach: bool = False,
    ) -> None:
        self._objectiveai_dir = objectiveai_dir
        self._extra_env = dict(extra_env) if extra_env else None
        self._kill_on_drop = kill_on_drop
        self._detach = detach

    async def execute(self, request: Any) -> AsyncIterator[Any]:
        argv = ["--request", json.dumps(request)]
        binary = self._resolve_binary()

        env = dict(os.environ)
        if self._extra_env:
            env.update(self._extra_env)

        kwargs: dict[str, Any] = {
            "stdin": asyncio.subprocess.DEVNULL,
            "stdout": asyncio.subprocess.PIPE,
            "stderr": None,  # inherit the parent's stderr
            "env": env,
        }
        if self._detach:
            # Detach so the child outlives the parent (the JS mirror's
            # `detached: true` + `child.unref()`).
            if sys.platform == "win32":
                kwargs["creationflags"] = 0x00000008  # DETACHED_PROCESS
            else:
                kwargs["start_new_session"] = True

        proc = await asyncio.create_subprocess_exec(binary, *argv, **kwargs)
        stdout = proc.stdout
        if stdout is None:
            raise RuntimeError("binary executor: child stdout was not piped")

        try:
            async for raw in stdout:
                line = raw.decode("utf-8", errors="replace").strip()
                if not line:
                    continue
                yield json.loads(line)
        finally:
            if self._kill_on_drop and proc.returncode is None:
                try:
                    proc.kill()
                except ProcessLookupError:
                    pass
            if not self._detach:
                try:
                    await proc.wait()
                except Exception:
                    pass

    def _resolve_binary(self) -> str:
        exe = "objectiveai.exe" if sys.platform == "win32" else "objectiveai"
        base = self._objectiveai_dir or os.path.join(str(Path.home()), ".objectiveai")
        return os.path.join(base, "bin", exe)
