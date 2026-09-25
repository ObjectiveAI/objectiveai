# Questions only Ronald can answer
Tracked by the build session. Maya does not field these. Newest findings first.

1. **Offers on the new wire — when?** Bounties need them. Today they exist only on the old HTTP/SSE daemon
   (`reports/legacy_endpoints.md`: `channels/publish`, `GET /channels`, first-wins accept).
2. **A verb to add, list and remove providers.** The app adds machines inside itself (Maya's call). Today
   a peer is a `config.yaml` entry read at start. Until a verb exists, `WireDaemon` would have to write the
   daemon's config file.
3. **How does a client learn an image's settings schema?** Each agentic-loop serves `GET /schema`, but only
   inside its container. The app currently compiles the six official schemas from source.
4. **How does a client learn a provider's volume names?** `agents::create` pins volumes by
   `volume_name`, "a name in the daemon's listing from that provider", but the daemon has no listing verb.
5. **How does a client reach the daemon** (address) **and what credential goes in the first frame?**
   Nothing implements the daemon side of `diverge-daemon-sdk` yet.
6. Images have no published names or digests yet. The app uses `diverge-agentic-loop-<kind>` with the
   digest `unbuilt`.
