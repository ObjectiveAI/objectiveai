import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('initialize → DELETE → POST returns 404; second DELETE also 404', async () => {
  rig = await startRig([]);

  // 1. initialize.
  const init = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
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
  const sessionId = init.headers.get('mcp-session-id');
  expect(sessionId).toBeTruthy();

  // 2. DELETE → 200.
  const del = await fetch(rig.proxy.url, {
    method: 'DELETE',
    headers: { 'Mcp-Session-Id': sessionId! },
  });
  expect(del.status).toBe(200);

  // 3. POST with the now-dead session id → 404.
  const post = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      'Mcp-Session-Id': sessionId!,
    },
    body: '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}',
  });
  expect(post.status).toBe(404);

  // 4. DELETE again → 404.
  const del2 = await fetch(rig.proxy.url, {
    method: 'DELETE',
    headers: { 'Mcp-Session-Id': sessionId! },
  });
  expect(del2.status).toBe(404);
});
