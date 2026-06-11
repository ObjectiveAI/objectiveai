# objectiveai-db

The ObjectiveAI database server — a thin, self-contained vehicle around
embedded PostgreSQL.

Running `objectiveai-db` ensures a postmaster is alive for the
`<CONFIG_BASE_DIR>/db/` cluster, bound to a fixed `ADDRESS`/`PORT`,
prints `listening on <addr>:<port>` to stderr, and exits — the
postmaster is daemonized and keeps running. The PostgreSQL archive is
bundled inside the binary, so there is nothing else to install.

Normally you don't run this directly: `objectiveai db spawn` launches
it using connection settings from `objectiveai config db ...`, and
`objectiveai db kill` stops the postmaster it started. Point
`config db` at any other PostgreSQL (a remote instance, a managed
service) and the ObjectiveAI CLI uses that instead — this binary is
only the zero-setup local option.

## Environment

| var | default | meaning |
|---|---|---|
| `CONFIG_BASE_DIR` | `~/.objectiveai` | state root (`db-bin/`, `db/`, `.pgpass`, `db.lock`) |
| `ADDRESS` | `127.0.0.1` | bind address |
| `PORT` | `5433` | bind port |
| `PASSWORD` | `objectiveai` | superuser password (applied on first initdb only) |
