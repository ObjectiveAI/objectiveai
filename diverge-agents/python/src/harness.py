# The harness: one turn of the agent's Python.
#
# Written to disk by the container at registration and run as
# `python3 harness.py agent.py` every turn, with one JSON object on
# stdin — `{"input": [...], "tools": [...], "resources": [...]}` —
# and one JSON envelope as the LAST line of stdout:
# `{"eval": <the last expression's value, or null>, "stdout": <what
# the script printed>}`.
#
# The contract is the CLI's (objectiveai-daemon's python harness),
# with the base64 gone: the source is parsed, and if its last
# statement is a bare expression that expression is split off, the
# rest is exec'd and it is eval'd — so `main()` on the last line makes
# the call's return value the turn. Everything runs in one globals
# dict with three names the script did not define: `input`, the whole
# conversation in its stored form — chunk objects, and each user
# message as an array of MCP content blocks; `tools`, the MCP tools; and
# `resources`, the MCP resources — both as listed this very turn.
#
# stdin is read to its end BEFORE anything else runs and before
# anything is written, so the container can feed it whole without a
# pipe filling in either direction.
#
# No safeguards, deliberately: a script that rebinds `json`, swaps
# `sys.stdout` back, or calls `sys.exit(0)` breaks the envelope, and
# the container reports that as the harness broken — the author's
# problem, not one this file defends against. An uncaught exception
# is a traceback on stderr and a nonzero exit, which the container
# reports as the script raising.
#
# `os._exit(0)` at the end, not a return: nothing the script left
# behind — an atexit hook, a daemon thread — may print after the
# envelope, and the envelope must be the last line.

import ast
import io
import json
import os
import sys

real_stdout = sys.stdout
feed = json.load(sys.stdin)
with open(sys.argv[1], encoding="utf-8") as source_file:
    source = source_file.read()
tree = ast.parse(source, sys.argv[1])

capture = io.StringIO()
sys.stdout = capture
scope = {
    "__name__": "__main__",
    "__builtins__": __builtins__,
    "input": feed["input"],
    "tools": feed["tools"],
    "resources": feed["resources"],
}
result = None
if tree.body and isinstance(tree.body[-1], ast.Expr):
    last = tree.body.pop()
    exec(compile(tree, sys.argv[1], "exec"), scope)
    result = eval(
        compile(ast.Expression(last.value), sys.argv[1], "eval"), scope
    )
else:
    exec(compile(tree, sys.argv[1], "exec"), scope)

sys.stdout = real_stdout
print(json.dumps({"eval": result, "stdout": capture.getvalue()}))
sys.stdout.flush()
os._exit(0)
