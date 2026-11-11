# Diverge

Diverge runs agents and tools in containers, on your own machines.

- **Set up from this repository:** [reports/SETUP_INSTRUCTIONS_1.md](reports/SETUP_INSTRUCTIONS_1.md)
- **Operate a running installation:** the skills under [diverge-skills](diverge-skills), starting with
  [diverge-skills/diverge/SKILL.md](diverge-skills/diverge/SKILL.md)
- **The provider protocol's specification:** [diverge-provider-web](diverge-provider-web)

| Folder | What it is |
|---|---|
| `diverge-daemon` | the daemon: accounts, agents, tools, volumes, the vault |
| `diverge-provider` | the provider: runs containers with podman, serves volumes and images |
| `diverge-container-proxy` | the proxy inside every container |
| `diverge-issuer` | an OpenID Connect issuer for identities |
| `diverge-postgres` | the supervisor of the daemon's own Postgres |
| `diverge-cli` | the `diverge` command line |
| `diverge-sdk-rs`, `diverge-sdk-js` | the SDKs |
| `diverge-agents` | the agent loop images: `cc`, `codex`, `hermes`, `openrouter`, `python` |
| `diverge-examples` | example agent and tool images |
| `diverge-world`, `diverge-viewer` | the desktop apps |
