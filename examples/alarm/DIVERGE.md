---
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Arguments",
  "description": "The arguments an alarm tool is made with.\n\nNone: every alarm names the agent that set it, and the tool is the\nsame whoever calls it. `{}` is the one value; any member is\nrefused, so a misspelled setting is never silently ignored.\n\n# How it runs\n\nAs a dependency tool: a service, declared by the image of every\nagent that sets alarms, granted `message` over the agents attached\nto it. A service, because it keeps time between the agents' runs;\nany scope serves, since every alarm wakes only its own agent.",
  "type": "object",
  "additionalProperties": false
}
---
# Alarm

Alarms an agent sets to wake itself later: once, at a time or after a
delay, or on a cron schedule. When an alarm is due, the tool messages
the agent that set it with the alarm's note, exactly this and nothing
else:

```text
<alarm>
{note}
</alarm>
```

The daemon starts the agent if it is not running, so an alarm wakes
an idle agent too. Three tools:

| tool | parameters | does |
|---|---|---|
| `set_alarm` | `note`, and exactly one of `at` (an RFC 3339 time), `in_seconds`, `cron` (five fields, UTC) | sets an alarm for the caller; answers its id and when it is first due |
| `list_alarms` | none | the caller's alarms, soonest first, each with its id, note, pattern if it recurs, when it is next due and when it was set |
| `cancel_alarm` | `alarm` | cancels one of the caller's alarms; a recurring one stops for good |

- **Every alarm wakes the agent that set it**, as the daemon attests
  the caller. No tool reaches another agent's alarms; another agent's
  alarm is answered as one that does not exist.
- **Cron** is five fields (minute, hour, day of month, month, day of
  week), read in UTC; a field for seconds or the year is refused.
- **An alarm that cannot be sent when due is dropped.** A one-shot
  alarm is gone; a recurring one moves on to its next time.
- **Overdue alarms fire once.** An alarm that fell due while the tool
  was down fires on the tool's first round after it comes back; a
  recurring one's missed repeats are skipped, never made up.

## Running it

The alarm tool is a **dependency tool**: a service, declared by the
image of every agent that sets alarms, at its registration. For the
agent containers in this repository that is an entry in the agent's
`dependencies` argument:

```json
{
  "image": { "digest": "sha256:…", "references": [{ "registry": "…", "name": "…" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "global",
  "permissions": [
    { "agents": { "actions": ["message"], "within": "self" } }
  ]
}
```

- **Arguments:** none; `{}`.
- **Secrets:** none.
- **Mode:** `service`. The tool keeps time between the agent's runs,
  so it is never `transient`, which stops with the run. A service is
  kept running while any agent that declares it is attached.
- **Scope:** any. Every alarm names the agent it wakes, so agents that
  share one tool never see or wake to each other's; `global` is one
  container for every agent that declares it.
- **Permissions:** `message` over `self`, the agents attached to the
  tool; that is the whole of what it asks the daemon.
- Only agents may use it; a client over a connect is refused.
