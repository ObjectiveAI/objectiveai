import { z } from 'zod';

type JsonValue = string | number | boolean | null | JsonValue[] | {
    [key: string]: JsonValue;
};

declare const ViewerDestinationSchema: z.ZodUnion<readonly [z.ZodLiteral<"objectiveai">, z.ZodObject<{
    plugin: z.ZodObject<{
        name: z.ZodString;
        owner: z.ZodString;
        version: z.ZodString;
    }, z.core.$strip>;
}, z.core.$strict>]>;
type ViewerDestination = z.infer<typeof ViewerDestinationSchema>;

declare const ViewerEventSchema: z.ZodUnion<readonly [z.ZodObject<{
    destination: z.ZodUnion<readonly [z.ZodLiteral<"objectiveai">, z.ZodObject<{
        plugin: z.ZodObject<{
            name: z.ZodString;
            owner: z.ZodString;
            version: z.ZodString;
        }, z.core.$strip>;
    }, z.core.$strict>]>;
    type: z.ZodLiteral<"inbound">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>, z.ZodObject<{
    destination: z.ZodUnion<readonly [z.ZodLiteral<"objectiveai">, z.ZodObject<{
        plugin: z.ZodObject<{
            name: z.ZodString;
            owner: z.ZodString;
            version: z.ZodString;
        }, z.core.$strip>;
    }, z.core.$strict>]>;
    id: z.ZodString;
    type: z.ZodLiteral<"cli_command">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>]>;
type ViewerEvent = z.infer<typeof ViewerEventSchema>;

/**
 * Register a handler for incoming `inbound` plugin events. Returns
 * an unsubscribe function.
 *
 * `sub_type` matches the `sub_type` field of the `Event::Inbound`
 * emitted by the Rust host — e.g. `plugins_run` for daemon-stream
 * run frames the host routes to this plugin's tab.
 *
 * In iframe context the events come from the host's bridge; in
 * standalone-dev context they come from `@tauri-apps/api`'s `listen`.
 */
declare function listen<T = unknown>(sub_type: string, handler: (value: T) => void): () => void;
/** Internal-use: clear in-flight state. Exposed for tests only.
 * Note: the module-level `message` event listener stays attached —
 * removing/re-attaching it would just register a duplicate. The
 * listeners map is what carries per-test state. */
declare function __resetForTests(): void;

export { type JsonValue as J, type ViewerDestination as V, __resetForTests as _, ViewerDestinationSchema as a, type ViewerEvent as b, ViewerEventSchema as c, listen as l };
