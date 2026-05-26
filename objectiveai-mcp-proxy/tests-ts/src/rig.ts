// Test rig for the TypeScript-side compliance suite.
//
// Spawns the proxy and N test upstreams as subprocesses (release builds
// at <workspace>/target/release/), then connects an
// `@modelcontextprotocol/sdk` Client through the proxy with whatever
// custom session-init headers the test wants.
//
// Run `cargo build --release -p objectiveai-mcp-proxy -p test-upstream`
// (or `objectiveai-mcp-proxy/test.sh`) before `pnpm test`.

import { spawn, type ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';

const READY_TIMEOUT_MS = 15_000;
const POLL_INTERVAL_MS = 50;

const __dirname = dirname(fileURLToPath(import.meta.url));
// __dirname is .../objectiveai-mcp-proxy/tests-ts/src; workspace root
// is three levels up.
const WORKSPACE = resolve(__dirname, '..', '..', '..');

export type ToolBehavior =
  | { kind: 'echo' }
  | { kind: 'sleep_then_echo'; duration_ms: number }
  | { kind: 'static'; reply: string };

export interface TestTool {
  name: string;
  description?: string;
  behavior: ToolBehavior;
}

export interface TestResource {
  uri: string;
  name?: string;
  text: string;
}

export interface UpstreamSpec {
  serverName: string;
  initialTools?: TestTool[];
  initialResources?: TestResource[];
  requireAuth?: string;
  headerGate?: { name: string; value: string };
}

export interface UpstreamHandle {
  url: string;
  controlBase: string;
  child: ChildProcess;
}

export interface ProxyHandle {
  url: string;
  child: ChildProcess;
}

export interface Rig {
  proxy: ProxyHandle;
  upstreams: UpstreamHandle[];
  /** JSON-stringified URL list suitable for the X-MCP-Servers header. */
  xMcpServers: () => string;
  /** Connect a fresh SDK client through the proxy with custom headers. */
  connectClient: (customHeaders: Record<string, string>) => Promise<Client>;
  /** Hit the test upstream's POST /__test/set-tools. */
  setUpstreamTools: (idx: number, tools: TestTool[]) => Promise<void>;
  /** Hit the test upstream's POST /__test/set-resources. */
  setUpstreamResources: (idx: number, resources: TestResource[]) => Promise<void>;
  /** Read the test upstream's GET /__test/seen-headers. */
  upstreamSeenHeaders: (idx: number) => Promise<Record<string, string>>;
  /** Tear down all spawned subprocesses. Idempotent. */
  cleanup: () => Promise<void>;
}

export async function startRig(specs: UpstreamSpec[]): Promise<Rig> {
  const upstreams: UpstreamHandle[] = [];
  for (const spec of specs) {
    upstreams.push(await spawnUpstream(spec));
  }
  const proxy = await spawnProxy();

  const xMcpServers = () => JSON.stringify(upstreams.map(u => u.url));

  const connectClient = async (
    customHeaders: Record<string, string>,
  ): Promise<Client> => {
    const transport = new StreamableHTTPClientTransport(
      new URL(proxy.url),
      {
        requestInit: { headers: customHeaders },
      },
    );
    const client = new Client({
      name: 'objectiveai-mcp-proxy-tests-ts',
      version: '2.0.11',
    });
    await client.connect(transport);
    return client;
  };

  const setUpstreamTools = async (idx: number, tools: TestTool[]) => {
    const url = `${upstreams[idx]!.controlBase}/set-tools`;
    const resp = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tools }),
    });
    if (!resp.ok) {
      throw new Error(`set-tools failed: ${resp.status} ${await resp.text()}`);
    }
  };

  const setUpstreamResources = async (idx: number, resources: TestResource[]) => {
    const url = `${upstreams[idx]!.controlBase}/set-resources`;
    const resp = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ resources }),
    });
    if (!resp.ok) {
      throw new Error(`set-resources failed: ${resp.status} ${await resp.text()}`);
    }
  };

  const upstreamSeenHeaders = async (idx: number) => {
    const url = `${upstreams[idx]!.controlBase}/seen-headers`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`seen-headers failed: ${resp.status}`);
    return (await resp.json()) as Record<string, string>;
  };

  const cleanup = async () => {
    for (const u of upstreams) {
      u.child.kill('SIGKILL');
    }
    proxy.child.kill('SIGKILL');
  };

  return {
    proxy,
    upstreams,
    xMcpServers,
    connectClient,
    setUpstreamTools,
    setUpstreamResources,
    upstreamSeenHeaders,
    cleanup,
  };
}

async function spawnUpstream(spec: UpstreamSpec): Promise<UpstreamHandle> {
  const port = await pickFreePort();
  const env: Record<string, string> = {
    ...process.env as Record<string, string>,
    ADDRESS: '127.0.0.1',
    PORT: String(port),
    SERVER_NAME: spec.serverName,
    INITIAL_TOOLS_JSON: JSON.stringify(spec.initialTools ?? []),
    INITIAL_RESOURCES_JSON: JSON.stringify(spec.initialResources ?? []),
  };
  if (spec.requireAuth) env.REQUIRE_AUTH = spec.requireAuth;
  if (spec.headerGate) {
    env.HEADER_GATE_NAME = spec.headerGate.name;
    env.HEADER_GATE_VALUE = spec.headerGate.value;
  }

  const child = spawn(binaryPath('test-upstream'), [], {
    env,
    stdio: ['ignore', 'ignore', 'ignore'],
  });
  await waitForListening('127.0.0.1', port);
  return {
    url: `http://127.0.0.1:${port}/`,
    controlBase: `http://127.0.0.1:${port}/__test`,
    child,
  };
}

async function spawnProxy(): Promise<ProxyHandle> {
  const port = await pickFreePort();
  const child = spawn(binaryPath('objectiveai-mcp-proxy'), [], {
    env: {
      ...process.env as Record<string, string>,
      ADDRESS: '127.0.0.1',
      PORT: String(port),
    },
    stdio: ['ignore', 'ignore', 'ignore'],
  });
  await waitForListening('127.0.0.1', port);
  return {
    url: `http://127.0.0.1:${port}/`,
    child,
  };
}

function pickFreePort(): Promise<number> {
  return new Promise((res, rej) => {
    const srv = createServer();
    srv.unref();
    srv.on('error', rej);
    srv.listen(0, '127.0.0.1', () => {
      const addr = srv.address();
      if (typeof addr !== 'object' || addr === null) {
        rej(new Error('no address'));
        return;
      }
      const port = addr.port;
      srv.close(() => res(port));
    });
  });
}

async function waitForListening(host: string, port: number): Promise<void> {
  const start = Date.now();
  while (true) {
    const ok = await tryConnect(host, port);
    if (ok) return;
    if (Date.now() - start > READY_TIMEOUT_MS) {
      throw new Error(`${host}:${port} did not start listening within ${READY_TIMEOUT_MS}ms`);
    }
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS));
  }
}

function tryConnect(host: string, port: number): Promise<boolean> {
  return new Promise(res => {
    const sock = new (require('node:net') as typeof import('node:net')).Socket();
    sock.setTimeout(500);
    sock.once('connect', () => {
      sock.destroy();
      res(true);
    });
    sock.once('error', () => {
      sock.destroy();
      res(false);
    });
    sock.once('timeout', () => {
      sock.destroy();
      res(false);
    });
    sock.connect(port, host);
  });
}

function binaryPath(name: string): string {
  const exeName = process.platform === 'win32' ? `${name}.exe` : name;
  for (const profile of ['release', 'debug']) {
    const candidate = join(WORKSPACE, 'target', profile, exeName);
    if (existsSync(candidate)) return candidate;
  }
  throw new Error(
    `could not find ${exeName} under ${WORKSPACE}/target/{release,debug}/. ` +
      `Run \`cargo build --release -p objectiveai-mcp-proxy -p test-upstream\` first.`,
  );
}
