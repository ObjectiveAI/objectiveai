import z from "zod";
import { Stream } from "./stream";
import { ObjectiveAIFetchError } from "./error";
import {
  viewerApiCallStream,
  viewerApiCallUnary,
  viewerApiCallUnaryNoResponse,
} from "./viewer/apiCallBridge";

/**
 * Read an environment variable, supporting both Node.js and Deno.
 */
function readEnv(env: string): string | undefined {
  if (typeof (globalThis as any).process !== "undefined") {
    return (globalThis as any).process.env?.[env]?.trim() ?? undefined;
  }
  if (typeof (globalThis as any).Deno !== "undefined") {
    return (globalThis as any).Deno.env?.get?.(env)?.trim();
  }
  return undefined;
}

/**
 * Truthy-ish env-var parser. Treats `"1"`, `"true"`, `"yes"`, `"on"`
 * (case-insensitive) as `true`. Anything else — including `undefined`
 * — is `false`. Matches the convention `OBJECTIVEAI_VIEWER=1` users
 * are likely to write.
 */
function isTruthyEnv(value: string | undefined): boolean {
  if (!value) return false;
  const v = value.toLowerCase();
  return v === "1" || v === "true" || v === "yes" || v === "on";
}

/**
 * Schema for ObjectiveAI client options.
 */
export const ObjectiveAIOptionsSchema = z
  .object({
    address: z
      .string()
      .nullish()
      .describe(
        "Base URL for the API. Falls back to OBJECTIVEAI_ADDRESS env var, then https://api.objectiveai.dev",
      ),
    authorization: z
      .string()
      .nullish()
      .describe("API key for authentication. Falls back to OBJECTIVEAI_AUTHORIZATION env var."),
    userAgent: z
      .string()
      .nullish()
      .describe("User-Agent header. Falls back to USER_AGENT env var."),
    httpReferer: z
      .string()
      .nullish()
      .describe("HTTP-Referer header. Falls back to HTTP_REFERER env var."),
    xTitle: z
      .string()
      .nullish()
      .describe("X-Title header. Falls back to X_TITLE env var."),
    xGithubAuthorization: z
      .string()
      .nullish()
      .describe("X-GITHUB-AUTHORIZATION header for GitHub-hosted function/profile access."),
    xOpenrouterAuthorization: z
      .string()
      .nullish()
      .describe("X-OPENROUTER-AUTHORIZATION header for BYOK (Bring Your Own Key) support."),
    xMcpAuthorization: z
      .record(z.string(), z.string())
      .nullish()
      .describe("X-MCP-AUTHORIZATION header (JSON-encoded map of MCP authorization headers)."),
    xViewerSignature: z
      .string()
      .nullish()
      .describe("X-VIEWER-SIGNATURE header for viewer authentication."),
    xViewerAddress: z
      .string()
      .nullish()
      .describe("X-VIEWER-ADDRESS header for viewer address."),
    xCommitAuthorName: z
      .string()
      .nullish()
      .describe("X-COMMIT-AUTHOR-NAME header for commit author name."),
    xCommitAuthorEmail: z
      .string()
      .nullish()
      .describe("X-COMMIT-AUTHOR-EMAIL header for commit author email."),
    agentId: z
      .string()
      .nullish()
      .describe(
        "X-OBJECTIVEAI-AGENT-ID header. Falls back to OBJECTIVEAI_AGENT_ID env var.",
      ),
    viewer: z
      .boolean()
      .nullish()
      .describe(
        "Route every HTTP request through the host viewer's `cli_run` " +
          "Tauri command (via the `cli-invoke` postMessage) instead of " +
          "`fetch()`. Requires running inside an objectiveai-viewer " +
          "plugin iframe; the host spawns `objectiveai api …` and streams " +
          "its JSONL output back. Falls back to OBJECTIVEAI_VIEWER env var " +
          "(any truthy value enables it).",
      ),
  })
  .describe("Options for the ObjectiveAI client.");
export type ObjectiveAIOptions = z.infer<typeof ObjectiveAIOptionsSchema>;

/**
 * Schema for request options.
 */
export const RequestOptionsSchema = z
  .object({
    headers: z
      .union([
        z.instanceof(Headers),
        z.record(z.string(), z.string()),
        z.array(z.tuple([z.string(), z.string()])),
      ])
      .nullish()
      .describe("Additional headers to include in the request."),
    signal: z
      .instanceof(AbortSignal)
      .nullish()
      .describe("AbortSignal for cancelling the request."),
  })
  .describe("Options for individual requests.");
export type RequestOptions = z.infer<typeof RequestOptionsSchema>;

/**
 * ObjectiveAI API client.
 */
export class ObjectiveAI {
  readonly address: string;
  readonly authorization: string | undefined;
  readonly userAgent: string | undefined;
  readonly httpReferer: string | undefined;
  readonly xTitle: string | undefined;
  readonly xGithubAuthorization: string | undefined;
  readonly xOpenrouterAuthorization: string | undefined;
  readonly xMcpAuthorization: Record<string, string> | undefined;
  readonly xViewerSignature: string | undefined;
  readonly xViewerAddress: string | undefined;
  readonly xCommitAuthorName: string | undefined;
  readonly xCommitAuthorEmail: string | undefined;
  readonly agentId: string | undefined;
  readonly viewer: boolean;

  constructor(options?: ObjectiveAIOptions | null) {
    this.address =
      options?.address ??
      readEnv("OBJECTIVEAI_ADDRESS") ??
      "https://api.objectiveai.dev";
    this.authorization =
      options?.authorization ?? readEnv("OBJECTIVEAI_AUTHORIZATION") ?? undefined;
    this.userAgent =
      options?.userAgent ?? readEnv("USER_AGENT") ?? undefined;
    this.httpReferer =
      options?.httpReferer ?? readEnv("HTTP_REFERER") ?? undefined;
    this.xTitle = options?.xTitle ?? readEnv("X_TITLE") ?? undefined;
    this.xGithubAuthorization =
      options?.xGithubAuthorization ?? readEnv("GITHUB_AUTHORIZATION") ?? undefined;
    this.xOpenrouterAuthorization =
      options?.xOpenrouterAuthorization ?? readEnv("OPENROUTER_AUTHORIZATION") ?? undefined;
    this.xMcpAuthorization =
      options?.xMcpAuthorization ?? (() => {
        const raw = readEnv("MCP_AUTHORIZATION");
        if (!raw) return undefined;
        try {
          const parsed = JSON.parse(raw);
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) return parsed;
        } catch {}
        return undefined;
      })();
    this.xViewerSignature =
      options?.xViewerSignature ?? readEnv("VIEWER_SIGNATURE") ?? undefined;
    this.xViewerAddress =
      options?.xViewerAddress ?? readEnv("VIEWER_ADDRESS") ?? undefined;
    this.xCommitAuthorName =
      options?.xCommitAuthorName ?? readEnv("COMMIT_AUTHOR_NAME") ?? undefined;
    this.xCommitAuthorEmail =
      options?.xCommitAuthorEmail ?? readEnv("COMMIT_AUTHOR_EMAIL") ?? undefined;
    this.agentId =
      options?.agentId ?? readEnv("OBJECTIVEAI_AGENT_ID") ?? undefined;
    this.viewer = options?.viewer ?? isTruthyEnv(readEnv("OBJECTIVEAI_VIEWER"));
  }

  /**
   * Build headers for a request.
   */
  private buildHeaders(options?: RequestOptions | null): Headers {
    const headers = new Headers();

    // Set default headers
    headers.set("Content-Type", "application/json");

    if (this.authorization) {
      headers.set("Authorization", `Bearer ${this.authorization}`);
    }
    if (this.userAgent) {
      headers.set("User-Agent", this.userAgent);
    }
    if (this.httpReferer) {
      headers.set("HTTP-Referer", this.httpReferer);
    }
    if (this.xTitle) {
      headers.set("X-Title", this.xTitle);
    }
    if (this.xGithubAuthorization) {
      headers.set("X-GITHUB-AUTHORIZATION", this.xGithubAuthorization);
    }
    if (this.xOpenrouterAuthorization) {
      headers.set("X-OPENROUTER-AUTHORIZATION", this.xOpenrouterAuthorization);
    }
    if (this.xMcpAuthorization) {
      headers.set("X-MCP-AUTHORIZATION", JSON.stringify(this.xMcpAuthorization));
    }
    if (this.xViewerSignature) {
      headers.set("X-VIEWER-SIGNATURE", this.xViewerSignature);
    }
    if (this.xViewerAddress) {
      headers.set("X-VIEWER-ADDRESS", this.xViewerAddress);
    }
    if (this.xCommitAuthorName) {
      headers.set("X-COMMIT-AUTHOR-NAME", this.xCommitAuthorName);
    }
    if (this.xCommitAuthorEmail) {
      headers.set("X-COMMIT-AUTHOR-EMAIL", this.xCommitAuthorEmail);
    }
    if (this.agentId) {
      headers.set("X-OBJECTIVEAI-AGENT-ID", this.agentId);
    }

    // Merge in request-specific headers
    if (options?.headers) {
      const optHeaders = options.headers;
      if (optHeaders instanceof Headers) {
        optHeaders.forEach((value, key) => headers.set(key, value));
      } else if (Array.isArray(optHeaders)) {
        for (const [key, value] of optHeaders) {
          headers.set(key, value);
        }
      } else {
        for (const [key, value] of Object.entries(optHeaders)) {
          headers.set(key, value);
        }
      }
    }

    return headers;
  }

  /**
   * Build the full URL for a path.
   */
  private buildUrl(path: string): string {
    const base = this.address.endsWith("/")
      ? this.address.slice(0, -1)
      : this.address;
    const normalizedPath = path.startsWith("/") ? path : `/${path}`;
    return `${base}${normalizedPath}`;
  }

  /**
   * Handle error responses, extracting the body.
   */
  private async handleErrorResponse(
    response: Response,
  ): Promise<ObjectiveAIFetchError> {
    let rawBody: string | null;
    try {
      rawBody = await response.text();
    } catch {
      rawBody = null;
    }

    return new ObjectiveAIFetchError(response.status, rawBody);
  }

  /**
   * Perform a GET request and return the parsed JSON response.
   */
  async get_unary<T>(
    path: string,
    options?: RequestOptions | null,
  ): Promise<T> {
    if (this.viewer) {
      return viewerApiCallUnary<T>("GET", path, undefined);
    }
    const response = await fetch(this.buildUrl(path), {
      method: "GET",
      headers: this.buildHeaders(options),
      signal: options?.signal ?? undefined,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return (await response.json()) as T;
  }

  /**
   * Perform a POST request and return the parsed JSON response.
   */
  async post_unary<T>(
    path: string,
    body?: unknown,
    options?: RequestOptions | null,
  ): Promise<T> {
    if (this.viewer) {
      return viewerApiCallUnary<T>("POST", path, body);
    }
    const response = await fetch(this.buildUrl(path), {
      method: "POST",
      headers: this.buildHeaders(options),
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: options?.signal ?? undefined,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return (await response.json()) as T;
  }

  /**
   * Perform a POST request that returns no body. Any 2xx status is
   * treated as success; non-2xx throws.
   */
  async post_unary_no_response(
    path: string,
    body?: unknown,
    options?: RequestOptions | null,
  ): Promise<void> {
    if (this.viewer) {
      return viewerApiCallUnaryNoResponse("POST", path, body);
    }
    const response = await fetch(this.buildUrl(path), {
      method: "POST",
      headers: this.buildHeaders(options),
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: options?.signal ?? undefined,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }
  }

  /**
   * Perform a DELETE request and return the parsed JSON response.
   */
  async delete_unary<T>(
    path: string,
    body?: unknown,
    options?: RequestOptions | null,
  ): Promise<T> {
    if (this.viewer) {
      return viewerApiCallUnary<T>("DELETE", path, body);
    }
    const response = await fetch(this.buildUrl(path), {
      method: "DELETE",
      headers: this.buildHeaders(options),
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: options?.signal ?? undefined,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return (await response.json()) as T;
  }

  /**
   * Perform a GET request and return an SSE stream.
   */
  async get_streaming<T>(
    path: string,
    options?: RequestOptions | null,
  ): Promise<Stream<T>> {
    if (this.viewer) {
      return viewerApiCallStream<T>("GET", path, undefined);
    }
    const headers = this.buildHeaders(options);
    headers.set("Accept", "text/event-stream");

    const controller = new AbortController();
    const signal = options?.signal;

    // Link external signal to our controller
    if (signal) {
      signal.addEventListener("abort", () => controller.abort());
    }

    const response = await fetch(this.buildUrl(path), {
      method: "GET",
      headers,
      signal: controller.signal,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return new Stream<T>(response, controller);
  }

  /**
   * Perform a POST request and return an SSE stream.
   */
  async post_streaming<T>(
    path: string,
    body?: unknown,
    options?: RequestOptions | null,
  ): Promise<Stream<T>> {
    if (this.viewer) {
      return viewerApiCallStream<T>("POST", path, body);
    }
    const headers = this.buildHeaders(options);
    headers.set("Accept", "text/event-stream");

    const controller = new AbortController();
    const signal = options?.signal;

    // Link external signal to our controller
    if (signal) {
      signal.addEventListener("abort", () => controller.abort());
    }

    const response = await fetch(this.buildUrl(path), {
      method: "POST",
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return new Stream<T>(response, controller);
  }

  /**
   * Perform a DELETE request and return an SSE stream.
   */
  async delete_streaming<T>(
    path: string,
    body?: unknown,
    options?: RequestOptions | null,
  ): Promise<Stream<T>> {
    if (this.viewer) {
      return viewerApiCallStream<T>("DELETE", path, body);
    }
    const headers = this.buildHeaders(options);
    headers.set("Accept", "text/event-stream");

    const controller = new AbortController();
    const signal = options?.signal;

    // Link external signal to our controller
    if (signal) {
      signal.addEventListener("abort", () => controller.abort());
    }

    const response = await fetch(this.buildUrl(path), {
      method: "DELETE",
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal: controller.signal,
    });

    if (!response.ok) {
      throw await this.handleErrorResponse(response);
    }

    return new Stream<T>(response, controller);
  }
}
