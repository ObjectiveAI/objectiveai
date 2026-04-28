import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('POST with insufficient Accept → 406', async () => {
  rig = await startRig([]);
  const resp = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'text/html' },
    body: '{"jsonrpc":"2.0","id":1,"method":"ping"}',
  });
  expect(resp.status).toBe(406);
});

test('POST with malformed JSON → 400 + JSON-RPC -32700 envelope', async () => {
  rig = await startRig([]);
  const resp = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: '{not json',
  });
  expect(resp.status).toBe(400);
  const body = (await resp.json()) as { jsonrpc: string; error: { code: number } };
  expect(body.jsonrpc).toBe('2.0');
  expect(body.error.code).toBe(-32700);
});

test('POST without Mcp-Session-Id → 404', async () => {
  rig = await startRig([]);
  const resp = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}',
  });
  expect(resp.status).toBe(404);
});

test('POST with unknown Mcp-Session-Id → 404', async () => {
  rig = await startRig([]);
  const resp = await fetch(rig.proxy.url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
      'Mcp-Session-Id': 'definitely-not-a-real-session',
    },
    body: '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}',
  });
  expect(resp.status).toBe(404);
});

test('initialize with wrong protocolVersion → 200 + JSON-RPC -32600 envelope', async () => {
  rig = await startRig([]);
  const resp = await fetch(rig.proxy.url, {
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
        protocolVersion: '1990-01-01',
        capabilities: {},
        clientInfo: { name: 't', version: '0' },
      },
    }),
  });
  expect(resp.status).toBe(200);
  const body = (await resp.json()) as { jsonrpc: string; error: { code: number } };
  expect(body.jsonrpc).toBe('2.0');
  expect(body.error.code).toBe(-32600);
});
