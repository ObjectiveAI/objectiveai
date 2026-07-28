/**
 * THE daemon client — the JS mirror of the Rust SDK's
 * `objectiveai_sdk::daemon::Client`: one handle minting every
 * per-endpoint structure (the materialized SSE listeners, the
 * laboratory file tree, the viewer-plugin bundle download, channel
 * accept) and acting as the `/execute` command executor itself — no
 * separate executor class.
 *
 * Two construction modes, every surface working in BOTH:
 *
 * - REGULAR (`new Client(address, options?)`) — fetch+SSE straight at
 *   the daemon's published `http://` address; the client stamps the
 *   auth signature and, on `/execute`, the caller identity headers.
 * - VIEWER (`Client.viewer(transport)`) — every surface rides the
 *   injected Tauri transport's `daemon_*` proxy commands; the
 *   viewer's Rust process owns address, signature, and identity —
 *   this mode stamps NOTHING.
 *
 * Listener methods are async and resolve only once the stream has
 * actually OPENED (2xx headers / proxy connect success) — a bad
 * address or a 401 is a rejection here, never a silently-frozen
 * view. One listener = one connection; reconnection is the caller's
 * loop.
 */

import type { CliCommandRequest } from "../cli/command/request";
import type { Identity } from "../identity";
import { AgentsInstancesListListener } from "./agentsInstancesListListener";
import { AgentsInstancesListener } from "./agentsInstancesListener";
import { ChannelListener } from "./channelListener";
import { CommandListener } from "./commandListener";
import { FileTree } from "./fileTree";
import { LaboratoriesListListener } from "./laboratoriesListListener";
import { LaboratoriesListener } from "./laboratoriesListener";
import type { ClientMode } from "./mode";
import { connectSse } from "./sse";
import {
  connectViewerStream,
  newStreamId,
  type ViewerTransport,
} from "./viewerStream";
import { ViewerPlugin, decodeBase64Chunk } from "./viewerPlugin";

export interface ClientOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent as
   * the `X-OBJECTIVEAI-SIGNATURE` header on every request. Without it
   * the daemon must be running without a secret. */
  signature?: string | null;
  /** The client's own caller identity — the DEFAULT for `/execute`'s
   * `X-OBJECTIVEAI-*` headers; a per-call identity wins over it. The
   * unspoofable `plugin_*` trio and `task` are NEVER stamped —
   * wire requests cannot assert them. */
  identity?: Identity;
}

/** Options for one {@link Client.execute} call. */
export interface ExecuteOptions {
  /** Per-call identity override — beats the client's own. */
  identity?: Identity;
}

/** The viewer-mode chunk event for `daemon_viewer_plugin` (binary
 * rides base64 — the text stream shape can't carry it). */
type ViewerPluginEvent =
  | { type: "chunk"; data: string }
  | { type: "end" }
  | { type: "error"; message: string };

export class Client {
  #mode: ClientMode;

  /** REGULAR mode: fetch+SSE straight at `address` (the daemon's
   * published base address, e.g. `http://127.0.0.1:49152`). */
  constructor(address: string, options?: ClientOptions) {
    this.#mode = {
      mode: "regular",
      address: address.replace(/\/+$/, ""),
      signature: options?.signature,
      identity: options?.identity,
    };
  }

  /** VIEWER mode: every surface rides the injected Tauri transport;
   * the Rust proxy owns address, signature, and identity. */
  static viewer(transport: ViewerTransport): Client {
    const client = new Client("");
    client.#mode = { mode: "viewer", transport };
    return client;
  }

  /** The resolved mode — @internal, consumed by the listener mints. */
  get _mode(): ClientMode {
    return this.#mode;
  }

  /**
   * Execute one CLI command — the client IS the executor (a drop-in
   * anywhere a `CommandExecutor` goes; the generated per-command
   * execute functions call this). One `POST /execute` (regular) or
   * one `daemon_execute` invoke (viewer) per call; the result is an
   * async iterable of the daemon's stream lines — a response JSON or
   * a `{"type":"error",…}` line; connect failures surface as one
   * in-band error line; `return()`/`break` cancels the daemon-side
   * run.
   */
  execute(
    request: CliCommandRequest,
    options?: ExecuteOptions,
  ): AsyncIterable<unknown> {
    const mode = this.#mode;
    return {
      async *[Symbol.asyncIterator]() {
        const controller = new AbortController();
        let events: AsyncGenerator<string>;
        try {
          if (mode.mode === "viewer") {
            events = await connectViewerStream(
              mode.transport,
              "daemon_execute",
              { request: JSON.stringify(request) },
              controller.signal,
            );
          } else {
            // Per-call identity beats the client's own; the plugin
            // trio + task never ride the wire (daemon-authored only).
            const identity = options?.identity ?? mode.identity;
            events = await connectSse(
              `${mode.address}/execute`,
              mode.signature,
              controller.signal,
              {
                method: "POST",
                body: JSON.stringify(request),
                agentInstanceHierarchy: identity?.agent_instance_hierarchy,
                agentId: identity?.agent_id,
                agentFullId: identity?.agent_full_id,
                agentRemote: identity?.agent_remote,
                responseId: identity?.response_id,
                responseIds: identity?.response_ids,
              },
            );
          }
        } catch (e) {
          yield {
            type: "error",
            level: "error",
            fatal: null,
            message: `connect daemon execute: ${String(e)}`,
          };
          return;
        }
        try {
          for await (const data of events) {
            try {
              yield JSON.parse(data);
            } catch (e) {
              yield {
                type: "error",
                level: "error",
                fatal: null,
                message: `decode daemon execute message: ${String(e)}`,
              };
            }
          }
        } catch (e) {
          yield {
            type: "error",
            level: "error",
            fatal: null,
            message: `daemon execute stream: ${String(e)}`,
          };
        } finally {
          // return()/break (or normal completion) aborts the request,
          // cancelling the daemon-side run.
          controller.abort();
        }
      },
    };
  }

  /** The `/listen` broadcast as a typed stream: every CLI execution
   * the daemon runs, request + response items, dispatched onto the
   * command tree's leaf types. */
  commandListener(): Promise<CommandListener> {
    return CommandListener._connect(this.#mode);
  }

  /** The `/agents/instances/list` view: every agent's active/inactive
   * status, live. */
  agentsInstancesListListener(): Promise<AgentsInstancesListListener> {
    return AgentsInstancesListListener._connect(this.#mode);
  }

  /** The `/agents/instances/{aih}` view: ONE agent's full
   * conversation, DB history + live rows. */
  agentsInstancesListener(
    agentInstanceHierarchy: string,
  ): Promise<AgentsInstancesListener> {
    return AgentsInstancesListener._connect(this.#mode, agentInstanceHierarchy);
  }

  /** The `/laboratories/list` view: the live laboratories merge
   * (connected ∪ local scan). */
  laboratoriesListListener(): Promise<LaboratoriesListListener> {
    return LaboratoriesListListener._connect(this.#mode);
  }

  /** The `/laboratories/{id}` view: ONE laboratory's record with
   * attachments. `machine`/`machineState` pin the exact owning host
   * (both or neither). */
  laboratoriesListener(
    laboratoryId: string,
    options?: { machine?: string; machineState?: string },
  ): Promise<LaboratoriesListener> {
    return LaboratoriesListener._connect(this.#mode, laboratoryId, options);
  }

  /** The `/laboratories/{id}/filetree` view: one laboratory's live
   * file tree. */
  fileTree(
    laboratoryId: string,
    options?: { machine?: string; machineState?: string },
  ): Promise<FileTree> {
    return FileTree._connect(this.#mode, laboratoryId, options);
  }

  /** The `/channels` offer-lifecycle view — answer its offers with
   * {@link acceptChannel}. */
  channelListener(): Promise<ChannelListener> {
    return ChannelListener._connect(this.#mode);
  }

  /**
   * Accept an open channel offer: a bare `POST /channels/{id}/accept`
   * (first-wins; `daemon_channel_accept` in viewer mode). Resolves
   * with the owner secret (`S_owner`) — the per-channel capability
   * for `channels logs reply|list|open|subscribe` and
   * `channels close`. Rejects on refusal — 404 unknown/withdrawn,
   * 409 already accepted, 401 unauthorized.
   */
  async acceptChannel(channelId: string): Promise<string> {
    if (this.#mode.mode === "viewer") {
      let secret: unknown;
      try {
        secret = await this.#mode.transport.invoke("daemon_channel_accept", {
          channelId,
        });
      } catch (e) {
        throw new Error(`channel accept: ${String(e)}`);
      }
      if (typeof secret !== "string") {
        throw new Error(`channel accept: proxy returned no secret`);
      }
      return secret;
    }
    const headers: Record<string, string> = {};
    if (this.#mode.signature) {
      headers["X-OBJECTIVEAI-SIGNATURE"] = this.#mode.signature;
    }
    let response: Response;
    try {
      response = await fetch(
        `${this.#mode.address}/channels/${encodeURIComponent(channelId)}/accept`,
        { method: "POST", headers },
      );
    } catch (e) {
      throw new Error(`channel accept: fetch failed: ${String(e)}`);
    }
    if (!response.ok) {
      throw new Error(`channel accept: HTTP ${response.status}`);
    }
    const body = await response.text();
    let accepted: { secret?: unknown };
    try {
      accepted = JSON.parse(body) as { secret?: unknown };
    } catch {
      throw new Error(`channel accept: unparseable body: ${body}`);
    }
    if (typeof accepted.secret !== "string") {
      throw new Error(`channel accept: body carried no secret: ${body}`);
    }
    return accepted.secret;
  }

  /**
   * Download a plugin's viewer-extension bundle:
   * `GET /plugins/{owner}/{name}/{version}/viewer` — the daemon
   * builds it on demand and streams tar.gz back
   * (`daemon_viewer_plugin` in viewer mode, base64 chunk events).
   * LOW-LEVEL by design — raw bytes, no filesystem; the caller
   * un-tars its own way. `version` is the plugin repo's v-prefixed
   * git tag (`v1.2.3`), byte-for-byte.
   */
  async getViewerPlugin(
    owner: string,
    name: string,
    version: string,
  ): Promise<ViewerPlugin> {
    if (this.#mode.mode === "viewer") {
      return this.#viewerModePlugin(this.#mode.transport, owner, name, version);
    }
    const headers: Record<string, string> = {};
    if (this.#mode.signature) {
      headers["X-OBJECTIVEAI-SIGNATURE"] = this.#mode.signature;
    }
    const url = `${this.#mode.address}/plugins/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/${encodeURIComponent(version)}/viewer`;
    const controller = new AbortController();
    let response: Response;
    try {
      response = await fetch(url, { headers, signal: controller.signal });
    } catch (e) {
      throw new Error(`get viewer plugin: fetch failed: ${String(e)}`);
    }
    if (!response.ok || !response.body) {
      const body = await response.text().catch(() => "");
      throw new Error(
        `get viewer plugin: HTTP ${response.status}${body ? `: ${body.trim()}` : ""}`,
      );
    }
    const commitSha = response.headers.get("x-objectiveai-sha");
    const reader = response.body.getReader();
    const chunks = (async function* () {
      try {
        for (;;) {
          const { value, done } = await reader.read();
          if (done) return;
          yield value;
        }
      } finally {
        reader.releaseLock();
        controller.abort();
      }
    })();
    return new ViewerPlugin(commitSha, chunks);
  }

  /** Viewer-mode download: `daemon_viewer_plugin` streams base64
   * `chunk` events over the standard streamId/onEvent shape and
   * resolves its invoke with `{ commitSha }`. */
  async #viewerModePlugin(
    transport: ViewerTransport,
    owner: string,
    name: string,
    version: string,
  ): Promise<ViewerPlugin> {
    const streamId = newStreamId();
    const queue: ViewerPluginEvent[] = [];
    let wake: (() => void) | null = null;
    const push = (event: ViewerPluginEvent) => {
      queue.push(event);
      if (wake !== null) {
        const w = wake;
        wake = null;
        w();
      }
    };
    const channel = transport.channel<ViewerPluginEvent>();
    channel.onmessage = push;
    let resolved: unknown;
    try {
      resolved = await transport.invoke("daemon_viewer_plugin", {
        owner,
        name,
        version,
        streamId,
        onEvent: channel,
      });
    } catch (e) {
      throw new Error(`get viewer plugin: ${String(e)}`);
    }
    const commitSha =
      typeof (resolved as { commitSha?: unknown } | null)?.commitSha ===
      "string"
        ? ((resolved as { commitSha: string }).commitSha ?? null)
        : null;
    const chunks = (async function* () {
      try {
        for (;;) {
          while (queue.length > 0) {
            const event = queue.shift()!;
            switch (event.type) {
              case "chunk":
                yield decodeBase64Chunk(event.data);
                break;
              case "end":
                return;
              case "error":
                throw new Error(event.message);
            }
          }
          await new Promise<void>((resolve) => {
            wake = resolve;
          });
        }
      } finally {
        void transport
          .invoke("daemon_stream_close", { streamId })
          .catch(() => {});
      }
    })();
    return new ViewerPlugin(commitSha, chunks);
  }
}
