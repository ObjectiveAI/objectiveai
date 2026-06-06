import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";
import {
  ViewerEventSchema,
  type ViewerEvent,
  CliCommandConfigAgentsFavoritesGetResponseItemSchema,
  type CliCommandConfigAgentsFavoritesGetResponseItem,
  AgentFavoritesChangedNotificationSchema,
} from "@objectiveai/sdk";

// Wire shape of the `agents favorites config get` cli-output
// notification. The cli emits via `emit_value(&favorites, ...)`
// which routes through `NotificationValue::Other`, so the wire shape
// is {"type":"notification","value":{"kind":"other","value":[<Favorite>, ...]}}.
const FavoritesListNotificationSchema = z.object({
  type: z.literal("notification"),
  value: z.object({
    kind: z.literal("other"),
    value: z.array(CliCommandConfigAgentsFavoritesGetResponseItemSchema),
  }),
});

export function useFavoriteAgents() {
  const [favorites, setFavorites] = useState<CliCommandConfigAgentsFavoritesGetResponseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const reqCounter = useRef(0);

  const refetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    const origin = `useFavoriteAgents-${++reqCounter.current}`;

    let unlisten: UnlistenFn | undefined;
    try {
      const result = await new Promise<CliCommandConfigAgentsFavoritesGetResponseItem[]>(
        (resolve, reject) => {
          listen<ViewerEvent>(origin, (ev) => {
            const parsed = ViewerEventSchema.safeParse(ev.payload);
            if (!parsed.success || parsed.data.type !== "cli_command") return;
            const note = FavoritesListNotificationSchema.safeParse(parsed.data.value);
            if (note.success) {
              resolve(note.data.value.value);
              return;
            }
            const line = parsed.data.value as { type?: string };
            if (line?.type === "error") {
              reject(new Error(JSON.stringify(line)));
            }
          })
            .then((fn) => {
              unlisten = fn;
            })
            .then(() =>
              invoke("cli_run", {
                args: ["agents", "favorites", "config", "get"],
                origin,
              }),
            )
            .catch(reject);
        },
      );
      setFavorites(result);
    } finally {
      unlisten?.();
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refetch().catch((e) => {
      setError(String(e));
      setLoading(false);
    });
    let unlisten: UnlistenFn | undefined;
    listen<ViewerEvent>("objectiveai", (ev) => {
      const parsed = ViewerEventSchema.safeParse(ev.payload);
      if (!parsed.success || parsed.data.type !== "inbound") return;
      if (parsed.data.sub_type !== "agents_favorites_changed") return;
      AgentFavoritesChangedNotificationSchema.safeParse(parsed.data.value);
      refetch().catch((e) => setError(String(e)));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [refetch]);

  return { favorites, loading, error, refetch };
}
