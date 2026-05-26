# Authoring a Local Tool

You're authoring a tool under `~/.objectiveai/tools/` by hand. The CLI
does not install anything on this path — it only hands you these
instructions. Follow the steps in order.

A tool is a single executable (compiled binary or script) that a host
agent can invoke. Unlike a plugin, a tool has no viewer UI, no
per-platform binary metadata, no GitHub install pipeline. It's a
manifest plus an executable, side-by-side in one flat directory.

## 1. Fetch the manifest schema

Don't guess the manifest fields. The CLI ships the live JSON Schema:

    objectiveai schemas filesystem tools Manifest get

Read all of it before writing anything to disk. The current shape is
two required string fields — `description` and `exec` — but always
defer to the schema in case it grew fields.

## 2. Write the manifest

Choose a tool name. The name is the filename stem of the manifest in
`~/.objectiveai/tools/`. Use lowercase ASCII letters, digits, `.`,
`_`, or `-`.

Create the manifest at:

    ~/.objectiveai/tools/<name>.json

With contents:

    {
      "description": "<one-line summary of what the tool does>",
      "exec": "<filename to invoke>"
    }

`description` is what host agents see when deciding whether to invoke
the tool — write it for that audience.

`exec` is a **filename**, not a path. It is resolved relative to
`~/.objectiveai/tools/` (the same directory the manifest sits in).
Include any platform-specific extension yourself — the CLI does not
synthesise one. Typical values: `mytool`, `mytool.exe`, `mytool.sh`,
`mytool.py`, `mytool.bat`.

## 3. Place the executable

Drop the executable at:

    ~/.objectiveai/tools/<exec>

…where `<exec>` is exactly the filename you wrote in the manifest's
`exec` field.

It can be a compiled binary OR a script. Hosts invoke it via
`Command::new(path)`, and the OS picks the right runner via shebang
(Unix) or extension association (Windows). For a shell script with a
shebang, set the executable bit on Unix:

    chmod +x ~/.objectiveai/tools/<exec>

## 4. Verify

List installed tools:

    objectiveai tools list

Your tool should appear with its manifest. Resolve to make sure the
exec exists:

    objectiveai tools get <name>

The emitted `tool` field includes the manifest's `description` and
`exec`. If `tool` is `null`, the manifest at `<name>.json` is missing
or malformed — re-check the schema.
