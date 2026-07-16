# objectiveai-cli

The user-facing `objectiveai` command-line client.

This is a thin WebSocket client: it clap-parses argv into a typed
`cli::command::Request`, ensures the resident `objectiveai-daemon` is
running, and ships the request over the daemon's `/execute` WebSocket.
The daemon runs every command in-process and streams the result back;
this client drains those JSON lines to stdout. All command logic,
state, the database, and the WebSocket server live in the
`objectiveai-daemon` crate.
