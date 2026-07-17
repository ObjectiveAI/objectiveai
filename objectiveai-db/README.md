# objectiveai-db

The ObjectiveAI database server — a thin, self-contained supervisor
around embedded PostgreSQL.

Running `objectiveai-db` provisions the postgres install (bundled
inside the binary — nothing else to install), initializes and starts
the `<objectiveai-dir>/state/<state>/db/` cluster on a random free
port, announces readiness on stdout as
`{"type":"ready","address":"postgresql://..."}`, and then stays
resident supervising the postmaster — the postmaster dies with the
supervisor.

Normally you don't run this directly: the `objectiveai` daemon spawns
it on demand as a leashed child (it dies with the daemon; `objectiveai
daemon kill` is the teardown) and reads the connection string off the
ready line. Point `config db` at any other PostgreSQL (a remote
instance, a managed service) and the CLI uses that instead — this
binary is only the zero-setup local option.

## Arguments

Everything rides argv (all three flags are required):

| flag | meaning |
|---|---|
| `--objectiveai-dir` | layout root (postgres install at `bin/pg-bin/`, shared by every state) |
| `--objectiveai-state` | state name; cluster at `state/<state>/db/` |
| `--pg-password` | superuser password (applied on first initdb only) |
