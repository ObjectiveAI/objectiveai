import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;

afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

const ACCEPT = 'application/json, text/event-stream';

async function initSession(proxyUrl: string, xMcpServers: string): Promise<string> {
  const resp = await fetch(proxyUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: ACCEPT,
      'X-MCP-Servers': xMcpServers,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-11-25',
        capabilities: {},
        clientInfo: { name: 'notify-test', version: '0.1.0' },
      },
    }),
  });
  expect(resp.ok, `initialize: ${resp.status}`).toBe(true);
  const sessionId = resp.headers.get('mcp-session-id');
  expect(sessionId).toBeTruthy();
  // drain body so the connection can be reused
  await resp.text();

  await fetch(proxyUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: ACCEPT,
      'mcp-session-id': sessionId!,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'notifications/initialized',
    }),
  });

  return sessionId!;
}

async function postNotify(
  proxyUrl: string,
  sessionId: string,
  body: unknown,
): Promise<Response> {
  return fetch(`${proxyUrl}notify`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'mcp-session-id': sessionId,
    },
    body: JSON.stringify(body),
  });
}

async function callTool(
  proxyUrl: string,
  sessionId: string,
  requestId: number,
  toolName: string,
): Promise<unknown> {
  const resp = await fetch(proxyUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: ACCEPT,
      'mcp-session-id': sessionId,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: requestId,
      method: 'tools/call',
      params: { name: toolName },
    }),
  });
  expect(resp.ok, `tools/call: ${resp.status}`).toBe(true);
  return parseJsonRpcBody(resp);
}

async function parseJsonRpcBody(resp: Response): Promise<any> {
  const text = await resp.text();
  try {
    return JSON.parse(text);
  } catch {
    const data = text
      .split('\n')
      .filter((l) => l.startsWith('data: '))
      .map((l) => l.slice('data: '.length))
      .join('');
    return JSON.parse(data);
  }
}

test('POST /notify wraps the next tools/call response with a system-reminder block', async () => {
  rig = await startRig([
    {
      serverName: 'alpha',
      initialTools: [
        { name: 'say', behavior: { kind: 'static', reply: 'tool-output' } },
      ],
    },
  ]);

  const sessionId = await initSession(rig.proxy.url, rig.xMcpServers());

  // Two notifies; both should land in arrival order in one wrapper.
  const r1 = await postNotify(rig.proxy.url, sessionId, [
    { type: 'text', text: 'first' },
  ]);
  expect(r1.status).toBe(202);
  const r2 = await postNotify(rig.proxy.url, sessionId, [
    { type: 'text', text: 'second' },
  ]);
  expect(r2.status).toBe(202);

  const first: any = await callTool(rig.proxy.url, sessionId, 2, 'alpha_say');
  const blocks = first.result.content as any[];
  expect(blocks).toHaveLength(5);
  expect(blocks[0].text).toBe(
    '<system-reminder>\nThe user sent a new message while you were working:\n',
  );
  expect(blocks[1].text).toBe('first');
  expect(blocks[2].text).toBe('second');
  expect(blocks[3].text).toBe('\n\n</system-reminder>');
  expect(blocks[4].text).toBe('tool-output');

  // Second tool call (no notify between) → no wrapper.
  const second: any = await callTool(rig.proxy.url, sessionId, 3, 'alpha_say');
  const drainedBlocks = second.result.content as any[];
  expect(drainedBlocks).toHaveLength(1);
  expect(drainedBlocks[0].text).toBe('tool-output');
});

test('POST /notify on an unknown session returns 404', async () => {
  rig = await startRig([
    {
      serverName: 'alpha',
      initialTools: [
        { name: 'say', behavior: { kind: 'static', reply: 'tool-output' } },
      ],
    },
  ]);

  const resp = await postNotify(rig.proxy.url, 'definitely-not-a-real-session', [
    { type: 'text', text: 'ghost' },
  ]);
  expect(resp.status).toBe(404);
});
