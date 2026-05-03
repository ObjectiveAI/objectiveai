import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('notifications/cancelled aborts an in-flight tools/call with -32800', async () => {
  rig = await startRig([
    {
      serverName: 'slow',
      initialTools: [
        { name: 'wait', behavior: { kind: 'sleep_then_echo', duration_ms: 5_000 } },
      ],
    },
  ]);

  // 1. initialize.
  const init = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      'X-MCP-Servers': rig.xMcpServers(),
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-06-18',
        capabilities: {},
        clientInfo: { name: 't', version: '0' },
      },
    }),
  });
  expect(init.status).toBe(200);
  const sessionId = init.headers.get('mcp-session-id')!;

  // 2. Kick off the slow call (id = 42). Don't await yet.
  const startedAt = Date.now();
  const callPromise = fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      'Mcp-Session-Id': sessionId,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 42,
      method: 'tools/call',
      params: { name: 'slow_wait', arguments: {} },
    }),
  });

  // 3. Give the proxy ~150ms to register the in-flight token.
  await new Promise(r => setTimeout(r, 150));

  // 4. Fire notifications/cancelled with the matching requestId.
  const cancel = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      'Mcp-Session-Id': sessionId,
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'notifications/cancelled',
      params: { requestId: 42, reason: 'test cancel' },
    }),
  });
  expect(cancel.status).toBe(202);

  const resp = await callPromise;
  const elapsed = Date.now() - startedAt;
  expect(elapsed).toBeLessThan(2_000); // way under the SleepThenEcho's 5s

  const body = (await resp.json()) as { jsonrpc: string; error: { code: number } };
  expect(body.jsonrpc).toBe('2.0');
  expect(body.error.code).toBe(-32800);
});
