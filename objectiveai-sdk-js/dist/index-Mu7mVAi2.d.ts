import { z } from 'zod';

type JsonValue = string | number | boolean | null | JsonValue[] | {
    [key: string]: JsonValue;
};

declare const ViewerEventSchema: z.ZodUnion<readonly [z.ZodObject<{
    destination: z.ZodString;
    sub_type: z.ZodString;
    type: z.ZodLiteral<"inbound">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>, z.ZodObject<{
    destination: z.ZodString;
    type: z.ZodLiteral<"cli_command">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>]>;
type ViewerEvent = z.infer<typeof ViewerEventSchema>;

/**
 * Register a handler for incoming `inbound` plugin events. Returns
 * an unsubscribe function.
 *
 * `sub_type` matches the `sub_type` field of the `Event::Inbound`
 * emitted by the Rust host — the string the plugin author
 * registered in their manifest's `viewer_routes` entry, or one of
 * the built-in event names (e.g. `agent_completions`).
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

export { type JsonValue as J, type ViewerEvent as V, __resetForTests as _, ViewerEventSchema as a, listen as l };
