import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;

afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('initialize handshake succeeds via the SDK client', async () => {
  rig = await startRig([{ serverName: 'upstream-a' }]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });
  // Connect resolves only after initialize completes; getServerVersion
  // returns the server's reported implementation info.
  expect(client.getServerVersion()).toBeDefined();
  await client.close();
});
