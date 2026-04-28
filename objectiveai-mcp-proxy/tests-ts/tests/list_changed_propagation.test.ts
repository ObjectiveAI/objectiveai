import { afterEach, expect, test } from 'vitest';

import { ToolListChangedNotificationSchema } from '@modelcontextprotocol/sdk/types.js';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('upstream tool changes propagate to the downstream client', async () => {
  rig = await startRig([
    {
      serverName: 'svc',
      initialTools: [{ name: 'alpha', behavior: { kind: 'echo' } }],
    },
  ]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });

  // Wire a notification handler that resolves a promise on first
  // notifications/tools/list_changed.
  let resolveChanged!: () => void;
  const changed = new Promise<void>(r => {
    resolveChanged = r;
  });
  client.setNotificationHandler(ToolListChangedNotificationSchema, async () => {
    resolveChanged();
  });

  // Initial state.
  const initial = new Set((await client.listTools()).tools.map(t => t.name));
  expect(initial).toEqual(new Set(['svc_alpha']));

  // Mutate the upstream — fires notifications/tools/list_changed onto
  // its session, the proxy forwards onto our session.
  await rig.setUpstreamTools(0, [
    { name: 'beta', behavior: { kind: 'echo' } },
    { name: 'gamma', behavior: { kind: 'echo' } },
  ]);

  // Wait at most 5s for the notification.
  const t = new Promise<never>((_, rej) =>
    setTimeout(() => rej(new Error('notifications/tools/list_changed never arrived')), 5_000),
  );
  await Promise.race([changed, t]);

  const after = new Set((await client.listTools()).tools.map(t => t.name));
  expect(after).toEqual(new Set(['svc_beta', 'svc_gamma']));

  await client.close();
});
