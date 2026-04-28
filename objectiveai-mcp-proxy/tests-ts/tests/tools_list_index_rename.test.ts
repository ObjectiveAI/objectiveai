import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('two upstreams with the same name get `<name>_<index>_<tool>`', async () => {
  rig = await startRig([
    { serverName: 'fs', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
    { serverName: 'fs', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
    { serverName: 'solo', initialTools: [{ name: 'Ping', behavior: { kind: 'echo' } }] },
  ]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['fs_0_Read', 'fs_1_Read', 'solo_Ping']));
  await client.close();
});
