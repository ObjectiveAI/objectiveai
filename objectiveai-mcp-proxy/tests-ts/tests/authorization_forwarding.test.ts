import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('missing X-MCP-Authorization → initialize fails with -32603', async () => {
  rig = await startRig([
    {
      serverName: 'private',
      initialTools: [{ name: 'hidden', behavior: { kind: 'echo' } }],
      requireAuth: 'Bearer secret',
    },
  ]);
  await expect(
    rig.connectClient({ 'X-MCP-Servers': rig.xMcpServers() }),
  ).rejects.toThrow(/-32603.*upstream connect failed/);
});

test('correct X-MCP-Authorization → upstream connects, tools appear', async () => {
  rig = await startRig([
    {
      serverName: 'private',
      initialTools: [{ name: 'hidden', behavior: { kind: 'echo' } }],
      requireAuth: 'Bearer secret',
    },
  ]);
  const url = rig.upstreams[0]!.url;
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
    'X-MCP-Headers': JSON.stringify({ [url]: { 'Authorization': 'Bearer secret' } }),
  });
  const tools = (await client.listTools()).tools.map(t => t.name);
  expect(tools).toEqual(['private_hidden']);
  await client.close();
});
