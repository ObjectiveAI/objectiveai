# `objectiveai-tests/`

Committed, per-test static on-disk data for every integration test
under `objectiveai-cli/tests/`. Each `#[test]` function gets its own
subdirectory, named after the function. The subdirectory mirrors
exactly what the test's `CONFIG_BASE_DIR` contains at runtime —
nothing more, nothing less.

## Layout

```
objectiveai-tests/
├── .gitignore
├── README.md
└── <test-fn-name>/                      one dir per #[test] function
    ├── plugins/                         → CONFIG_BASE_DIR/plugins
    │   ├── <name>.json                  plugin manifest, static
    │   └── <name>/.gitkeep              binary slot — built binary
    │                                    copied here at runtime
    └── tools/                           → CONFIG_BASE_DIR/tools
        ├── <name>.json                  tool manifest, static
        └── <name>/.gitkeep              binary slot, when needed
```

## Rules

- One subdirectory per `#[test]` function, named to match the source
  function name exactly. Matches `cli_test_util::test_base_dir()`,
  which keys off the test thread name.
- Only `plugins/` and `tools/` at the top of a `<test-fn-name>/`
  directory — those are the only paths the cli's `filesystem::Client`
  reads from `CONFIG_BASE_DIR`.
- `.gitkeep` marks a directory whose contents are deposited at test
  runtime. Most are binary slots; a top-level `.gitkeep` marks a
  subdir with no static on-disk data (snapshot tests, viewer test).
- No built binaries (`*.exe`, `*.dylib`, etc.). The runtime
  fixture-copy step slots them in from `target/test-cli/debug/`.

## What does NOT live here

- Inline agent specs, profiles, and request bodies — those are
  constructed in Rust (`json!` / typed builders) and passed to the
  cli over stdin. They are not on-disk data.
- Runtime state the cli writes (`logs/`, `pipes/`, `plugin-pid`,
  ephemeral `*.txt` outputs from fixtures). Tracked by the
  `.gitignore` at this level.
- Built binaries.

## Status

This tree is committed but not yet consumed. Tests today still
hand-stage their `CONFIG_BASE_DIR` in `cli_test_util.rs` and
per-test `stage_*` helpers. A follow-up pass wires
`cli_test_util::test_base_dir()` to seed from this tree; a later
pass eliminates the runtime staging entirely.
