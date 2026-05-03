import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('correct X-MCP-Headers value lets upstream through', async () => {
  rig = await startRig([
    {
      serverName: 'gated',
      initialTools: [{ name: 'ok', behavior: { kind: 'echo' } }],
      headerGate: { name: 'X-Trace-Id', value: 'abc' },
    },
  ]);
  const url = rig.upstreams[0]!.url;
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
    'X-MCP-Headers': JSON.stringify({ [url]: { 'X-Trace-Id': 'abc' } }),
  });
  const tools = (await client.listTools()).tools.map(t => t.name);
  expect(tools).toEqual(['gated_ok']);

  const seen = await rig.upstreamSeenHeaders(0);
  expect(seen['x-trace-id']).toBe('abc');

  await client.close();
});

test('wrong X-MCP-Headers value fails initialize with -32603', async () => {
  rig = await startRig([
    {
      serverName: 'gated',
      initialTools: [{ name: 'ok', behavior: { kind: 'echo' } }],
      headerGate: { name: 'X-Trace-Id', value: 'abc' },
    },
  ]);
  const url = rig.upstreams[0]!.url;
  await expect(
    rig.connectClient({
      'X-MCP-Servers': rig.xMcpServers(),
      'X-MCP-Headers': JSON.stringify({ [url]: { 'X-Trace-Id': 'wrong' } }),
    }),
  ).rejects.toThrow(/-32603.*upstream connect failed/);
});
