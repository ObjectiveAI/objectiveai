# Authoring a Local Tool

You're authoring a tool under `~/.objectiveai/tools/` by hand. The CLI
does not install anything on this path — it only hands you these
instructions. Follow the steps in order.

A tool is an executable command (compiled binary or script, plus any
leading arguments) that a host agent can invoke. Unlike a plugin, a
tool has no viewer UI, no per-platform binary download, no GitHub
install pipeline. It's a manifest (`objectiveai.json`) sitting in a
versioned directory, alongside whatever files its exec command needs.

## 1. Fetch the manifest schema

Don't guess the manifest fields. The CLI ships the live JSON Schema:

    objectiveai schemas filesystem tools Manifest get

Read all of it before writing anything to disk. The current shape is
`description`, `version`, `owner`, and `exec` (a per-OS command
object) — but always defer to the schema in case it grew fields.

## 2. Choose a coordinate

A tool is addressed by `(owner, name, version)` and lives in its own
directory:

    ~/.objectiveai/tools/<owner>/<name>/<version>/

Use lowercase ASCII letters, digits, `.`, `_`, or `-` for each
segment.

## 3. Write the manifest

Create the manifest at:

    ~/.objectiveai/tools/<owner>/<name>/<version>/objectiveai.json

With contents:

    {
      "description": "<one-line summary of what the tool does>",
      "version": "<version>",
      "owner": "<owner>",
      "exec": {
        "windows": ["<program>", "<arg>", ...],
        "linux":   ["<program>", "<arg>", ...],
        "macos":   ["<program>", "<arg>", ...]
      }
    }

`description` is what host agents see when deciding whether to invoke
the tool — write it for that audience.

`exec` is a **per-OS command vector**. At run time the CLI picks the
vector for the current platform, appends the caller's `--args`, and
invokes the result **with this version directory as the working
directory**. So the first element is the program (looked up on `PATH`,
or `./relative` to this folder), and the rest are its leading
arguments. Leave a platform's list empty (`[]`) if the tool doesn't
support it.

Examples:

- A Python script shipped alongside the manifest:
  `["python", "tool.py"]` (drop `tool.py` in the same directory).
- A committed binary: `["./mytool"]` on linux/macos,
  `["mytool.exe"]` on windows.

## 4. Place any exec files

Drop whatever your exec command references (scripts, binaries) into the
same version directory, since that's the working directory at run
time. For a shell script with a shebang, set the executable bit on
Unix:

    chmod +x ~/.objectiveai/tools/<owner>/<name>/<version>/<file>

## 5. Verify

List installed tools:

    objectiveai tools list

Your tool should appear with its manifest. Then fetch it by coordinate:

    objectiveai tools get --owner <owner> --name <name> --version <version>

If the result is `null`, the manifest at
`<owner>/<name>/<version>/objectiveai.json` is missing or malformed —
re-check the schema.
