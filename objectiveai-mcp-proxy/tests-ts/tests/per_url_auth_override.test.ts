import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('per-URL X-MCP-Authorization map lets each upstream see its own bearer', async () => {
  rig = await startRig([
    {
      serverName: 'alpha',
      initialTools: [{ name: 'aTool', behavior: { kind: 'echo' } }],
      requireAuth: 'Bearer A',
    },
    {
      serverName: 'beta',
      initialTools: [{ name: 'bTool', behavior: { kind: 'echo' } }],
      requireAuth: 'Bearer B',
    },
  ]);
  const headersMap = {
    [rig.upstreams[0]!.url]: { 'Authorization': 'Bearer A' },
    [rig.upstreams[1]!.url]: { 'Authorization': 'Bearer B' },
  };
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
    'X-MCP-Headers': JSON.stringify(headersMap),
  });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['alpha_aTool', 'beta_bTool']));
  await client.close();
});
